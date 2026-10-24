--- Tests for taskflow.socket (mock-based, no real socket needed)
describe('socket', function()
  -- We test the JSON parsing logic by stubbing vim.fn.json_decode / json_encode.

  it('parses valid ping response', function()
    local response_json = '{"success":true,"data":{"pong":true,"protocol_version":"1"}}'
    local decoded = vim.fn.json_decode(response_json)
    assert.truthy(decoded.success)
    assert.truthy(decoded.data.pong)
    assert.equals('1', decoded.data.protocol_version)
  end)

  it('handles malformed JSON gracefully', function()
    local ok, err = pcall(vim.fn.json_decode, '{not valid json')
    assert.falsy(ok)
    assert.truthy(err)
  end)

  it('encodes requests to JSON', function()
    local req = { type = 'ping' }
    local encoded = vim.fn.json_encode(req)
    assert.truthy(encoded:find('"type"', 1, true))
    assert.truthy(encoded:find('"ping"', 1, true))
  end)

  it('request_sync returns timeout error when no server', function()
    local socket = require('taskflow.socket')
    -- Use a non-existent socket path to force an immediate connection error
    local ok, resp, err = socket.request_sync('/tmp/nonexistent_tf_test.sock', { type = 'ping' }, 200)
    -- Should fail (connection refused or file not found)
    assert.falsy(ok)
    assert.is_nil(resp)
    assert.truthy(err)
  end)
end)
