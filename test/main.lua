print("Meowing on " .. wm.name)

wm.input:on("new_keyboard", function(keyboard)
	print("New keyboard added!")
	keyboard:set_layout("tr")
end)

wm:on("ready", function()
	print("WM is ready!")
	spawn("alacritty")
	spawn("dms run")
end)

wm.input:on("key", function(e)
	print("KEY EVENT: " .. e.key)

	-- Alt + T
	if e.modifiers.alt and e.key == Key.T then
		print("Alt + T pressed")
		spawn("alacritty")
		e:prevent_default()
	end

	-- Alt + R
	if e.modifiers.alt and e.key == Key.R then
		wm.outputs:name("winit"):set_position({ x = 0, y = 0 })
		e:prevent_default()
	end

	if e.modifiers.alt and e.modifiers.shift and e.key == Key.Q then
		wm:quit()
	end

	local deltamap = {
		[tostring(Key.Up)] = { x = 0, y = -10 }, -- Up
		[tostring(Key.Down)] = { x = 0, y = 10 }, -- Down
		[tostring(Key.Left)] = { x = -10, y = 0 }, -- Left
		[tostring(Key.Right)] = { x = 10, y = 0 }, -- Right
	}

	local delta = deltamap[tostring(e.key)]

	print("Modifiers: " ..
		(e.modifiers.alt and "Alt " or "") ..
		(e.modifiers.shift and "Shift " or "") ..
		(e.modifiers.ctrl and "Ctrl " or "") .. (e.modifiers.logo and "Logo " or ""))

	if e.modifiers.alt and delta then
		e:prevent_default()
		print("Moving output")
		local output = wm.outputs:get(1)
		if not output then
			print("Output not found")
			return
		end
		local pos = output:position()
		print("Current position: " .. pos.x .. ", " .. pos.y)
		output:set_position({
			x = pos.x + delta.x,
			y = pos.y + delta.y
		})
	end
end)

wm.input:on("pointer_move", function(e)
	if e.pointer:buttons() then
		if e.pointer:buttons()[1] then
			local window = e.pointer:window_under()
			if window then
				e:prevent_default()
				local pos = window:position()
				window:set_position({
					x = pos.x + e.delta.x,
					y = pos.y + e.delta.y
				})
			else
				local output = wm.outputs:get(1)
				if output then
					e:prevent_default()
					local pos = output:position()
					output:set_position({
						x = pos.x - e.delta.x,
						y = pos.y - e.delta.y
					})
				end
			end
		end
	end

	return

	-- constrain pointer to the total bounding box of all outputs
	local bounds
	for _, out in ipairs(wm.outputs:all()) do
		local r = Rect(out:position().x, out:position().y, out:size().width, out:size().height)
		bounds = bounds and bounds:merge(r) or r
	end

	if bounds then
		local pos = e.position
		local clamped = {
			x = math.max(bounds.x, math.min(bounds.x + bounds.width - 1, pos.x)),
			y = math.max(bounds.y, math.min(bounds.y + bounds.height - 1, pos.y)),
		}
		if clamped.x ~= pos.x or clamped.y ~= pos.y then
			e:prevent_default()
			e.pointer:move(clamped)
		end
	end
end)

wm.windows:on("new_window", function(window)
	print("New window added!")
end)
