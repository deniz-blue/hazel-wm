local M = {}

local last_used = nil

--- The seat that most recently got a key or pointer event, or the first seat before any has.
function M.active()
	return last_used or wm.input.seats:all()[1]
end

--- The window a seat has focused, or nil when it has none or a layer. Defaults to the active seat.
function M.focused_window(seat)
	local target = (seat or M.active()):focused()
	return target and target:kind() == "window" and target or nil
end

local function note_used(seat)
	if seat then last_used = seat end
end

wm.input:on("key", function(event)
	note_used(event.keyboard:seat())
end)

wm.input:on("pointer_move", function(event)
	note_used(event.pointer:seat())
end)

wm.input:on("pointer_button", function(event)
	note_used(event.pointer:seat())
end)

return M
