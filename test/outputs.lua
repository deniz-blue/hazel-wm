local M = {}

wm.store.outputs = wm.store.outputs or { reserved = {} }
local state = wm.store.outputs
local last_pointer_output = {}

local function remember(output)
	if output then state.active = output.name end
end

--- The output that most recently got attention: a pointer moving onto it, or a window on it getting focus.
function M.active()
	return (state.active and wm.outputs:name(state.active)) or wm.outputs:get(1)
end

--- Keeps `top` pixels at the top of the output free of windows, as a layer-shell panel would.
function M.reserve_top(output, top)
	state.reserved[output.name] = top > 0 and top or nil
end

--- `output:usable_area()` less what `reserve_top` took.
function M.usable_area(output)
	local area = output:usable_area()
	local top = state.reserved[output.name] or 0
	return Rect(area.x, area.y + top, area.width, math.max(area.height - top, 0))
end

wm.input:on("pointer_move", function(event)
	local seat = event.pointer:seat()
	local under = wm.outputs:under(event.position)[1]
	local name = seat and seat.name
	if not (under and name) or last_pointer_output[name] == under.name then return end
	last_pointer_output[name] = under.name
	remember(under)
end)

wm.windows:on("focus", function(window)
	if window then remember(window:output()) end
end)

wm.outputs:on("removed", function(output)
	state.reserved[output.name] = nil
	last_pointer_output = {}
	if state.active == output.name then state.active = nil end
end)

return M
