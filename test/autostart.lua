local session = require("session")
local M = {}

--- Runs `program` once per session. A reload does not start it again.
function M.once(program, args, options)
	if wm.is_reload then return end
	session.spawn(program, args, options)
end

return M
