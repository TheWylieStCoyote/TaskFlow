--- Unix Domain Socket I/O for the TaskFlow Neovim plugin.
-- Uses vim.uv (Neovim 0.10+) or vim.loop (older) for non-blocking I/O.
--
-- libuv callbacks run in a "fast event" context where `vim.fn.*` and most
-- editor APIs are forbidden (E5560). JSON goes through `vim.json`, which is
-- safe there, and the caller's callback is always delivered on the main loop
-- via `vim.schedule`.
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
  local finished = false
  -- Deliver exactly one result, on the main loop.
  local function finish(ok, resp, err)
    if finished then return end
    finished = true
    vim.schedule(function() callback(ok, resp, err) end)
  end

  local encoded_ok, payload = pcall(vim.json.encode, req_table)
  if not encoded_ok then
    finish(false, nil, 'json_encode: ' .. tostring(payload))
    return
  end
  payload = payload .. '\n'

  local pipe = uv.new_pipe(false)
  if not pipe then
    finish(false, nil, 'Failed to create pipe')
    return
  end

  local buf = ''

  pipe:connect(socket_path, function(err)
    if err then
      pipe:close()
      finish(false, nil, 'connect: ' .. err)
      return
    end

    -- Start reading the response
    pipe:read_start(function(read_err, data)
      if read_err then
        pipe:close()
        finish(false, nil, 'read: ' .. read_err)
        return
      end
      if data then
        buf = buf .. data
      else
        -- EOF: connection closed by server
        pipe:close()
        local line = buf:match('^([^\n]*)')
        if not line or line == '' then
          finish(false, nil, 'empty response')
          return
        end
        -- JSON null becomes Lua nil (not vim.NIL) so callers can test
        -- optional fields with a plain `if`.
        local ok, decoded = pcall(vim.json.decode, line, { luanil = { object = true, array = true } })
        if ok then
          finish(true, decoded, nil)
        else
          finish(false, nil, 'json_decode: ' .. tostring(decoded))
        end
      end
    end)

    -- Write the request (encoded above, outside the fast context)
    pipe:write(payload, function(write_err)
      if write_err then
        pipe:close()
        finish(false, nil, 'write: ' .. write_err)
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
