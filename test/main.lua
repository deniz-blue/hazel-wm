local tiling = require("tiling")

tiling.add_workspace("4", "horizontal")
tiling.add_workspace("5", "vertical")
tiling.active("1")

-- ── Keyboard layout ───────────────────────────────────────────────

wm.input:on("new_keyboard", function(keyboard)
	keyboard:set_layout("tr")
end)

-- ── Workspace cycling ───────────────────────────────────────────────

local active_ws = "1"

local function cycle_workspace(dir)
	local list = tiling.list()
	if #list == 0 then return end

	local idx
	for i, ws in ipairs(list) do
		if ws.name == active_ws then idx = i; break end
	end

	local n
	if dir > 0 then
		n = idx and (idx % #list) + 1 or 1
	else
		n = idx and ((idx + #list - 2) % #list) + 1 or #list
	end

	active_ws = list[n].name
	tiling.active(active_ws)
end

-- ── Keybinds ────────────────────────────────────────────────────────

wm.input:on("key", function(e)
	if e.state ~= "Pressed" then return end

	-- Super + PageUp / PageDown → cycle workspace
	if e.modifiers.logo then
		if e.key == Key.Page_Up then
			cycle_workspace(1)
			e:prevent_default()
			return
		end
		if e.key == Key.Page_Down then
			cycle_workspace(-1)
			e:prevent_default()
			return
		end
	end

	-- Super + Esc → quit
	if e.modifiers.logo and e.key == Key.Escape then
		wm:quit()
		e:prevent_default()
		return
	end

	-- Super + T → kitty
	if e.modifiers.logo and e.key == Key.T then
		spawn("alacritty")
		e:prevent_default()
		return
	end

	-- Super + W → firefox-developer-edition
	if e.modifiers.logo and e.key == Key.W then
		spawn("firefox-developer-edition")
		e:prevent_default()
		return
	end

	-- Super + R → force retile all workspaces
	if e.modifiers.logo and e.key == Key.R then
		tiling.retile()
		tiling.debug()
		e:prevent_default()
		return
	end
end)
