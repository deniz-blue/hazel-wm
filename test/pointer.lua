local lock = require("lock")
local seats = require("seats")
local M = {}

local constraints = {}

local function only_alive(list)
	local kept = {}
	for _, constraint in ipairs(list) do
		if constraint:alive() then table.insert(kept, constraint) end
	end
	return kept
end

local function wanted(constraint, position)
	local window = constraint:window()
	if window == nil or window ~= seats.focused_window(constraint:pointer():seat()) then return false end
	return constraint:active() or wm.input:target_under(position) == window
end

local function update(pointer, position)
	constraints = only_alive(constraints)
	for _, constraint in ipairs(constraints) do
		local want = wanted(constraint, position)
		if want and not constraint:active() then
			constraint:activate()
		elseif not want and constraint:active() then
			constraint:deactivate()
		end
	end
end

wm.input:on("pointer_constraint", function(constraint)
	table.insert(constraints, constraint)
	local pointer = constraint:pointer()
	update(pointer, pointer:position())
end)

wm.input:on("pointer_move", lock.unless_locked(function(event)
	if #constraints > 0 then update(event.pointer, event.position) end
end))

wm.windows:on("focus", function()
	for _, constraint in ipairs(constraints) do
		local pointer = constraint:pointer()
		update(pointer, pointer:position())
	end
end)

return M
