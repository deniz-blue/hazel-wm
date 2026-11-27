local M = {}

local style = { width = 2, focused = "#88c0d0", unfocused = "#4c566a" }

local function color_of(window)
	return (#window:focused_seats() > 0) and style.focused or style.unfocused
end

local function wants_border(window)
	local states = window:states()
	return window:mapped() and not (states:has("fullscreen") or states:has("maximized"))
end

function M.refresh()
	for _, window in ipairs(wm.windows:all()) do
		if wants_border(window) then
			window:set_border(style.width, color_of(window))
		else
			window:clear_border()
		end
	end
end

function M.setup(options)
	for name, value in pairs(options) do style[name] = value end
	M.refresh()
end

wm.windows:on("mapped", function() M.refresh() end)
wm.windows:on("focus", function() M.refresh() end)

return M
