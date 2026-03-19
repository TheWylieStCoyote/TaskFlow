--- Unix Domain Socket I/O for the TaskFlow Neovim plugin.
-- Uses vim.uv (Neovim 0.10+) or vim.loop (older) for non-blocking I/O.
local M = {}

local uv = vim.uv or vim.loop

--- Send one JSON request over the socket and call back with the response.
---
--- The callback signature is:  callback(ok, response_table, err_string)
---   ok            -- boolean: true on success
---   response_table -- decoded JSON table (only when ok == true)
---   err_string    -- human-readable error (only when ok == false)
---
---@param socket_path string
---@param req_table   table   request object (will be JSON-encoded)
---@param callback    function
function M.request(socket_path, req_table, callback)
  local pipe = uv.new_pipe(false)
  if not pipe then
    callback(false, nil, 'Failed to create pipe')
    return
  end

  local buf = ''

  pipe:connect(socket_path, function(err)
    if err then
      pipe:close()
      callback(false, nil, 'connect: ' .. err)
      return
    end

    -- Start reading the response
    pipe:read_start(function(read_err, data)
      if read_err then
        pipe:close()
        callback(false, nil, 'read: ' .. read_err)
        return
      end
      if data then
        buf = buf .. data
      else
        -- EOF: connection closed by server
        pipe:close()
        local line = buf:match('^([^\n]*)')
        if not line or line == '' then
          callback(false, nil, 'empty response')
          return
        end
        local ok, decoded = pcall(vim.fn.json_decode, line)
        if ok then
          callback(true, decoded, nil)
        else
          callback(false, nil, 'json_decode: ' .. tostring(decoded))
        end
      end
    end)

    -- Write the request
    local payload = vim.fn.json_encode(req_table) .. '\n'
    pipe:write(payload, function(write_err)
      if write_err then
        -- Attempt graceful shutdown; read_start callback will fire with error
        pipe:close()
        callback(false, nil, 'write: ' .. write_err)
      end
    end)
  end)
end

--- Synchronous wrapper around `M.request`.
-- Blocks the calling coroutine (or the event loop via vim.wait) until the
-- request completes or the timeout expires.
---
---@param socket_path string
---@param req_table   table
---@param timeout_ms  integer
---@return boolean ok
---@return table|nil response
---@return string|nil err
function M.request_sync(socket_path, req_table, timeout_ms)
  local done = false
  local result_ok, result_resp, result_err

  M.request(socket_path, req_table, function(ok, resp, err)
    result_ok   = ok
    result_resp = resp
    result_err  = err
    done = true
  end)

  vim.wait(timeout_ms, function() return done end, 10)

  if not done then
    return false, nil, 'timeout after ' .. timeout_ms .. 'ms'
  end
  return result_ok, result_resp, result_err
end

return M
