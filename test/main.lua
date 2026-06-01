print("Meowing on " .. wm.name)

-- wm.input.keyboard:set_layout("tr")

wm:on("ready", function()
	print("WM is ready!")
	spawn("alacritty")
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

	local deltamap = {
		[tostring(Key.Up)] = { x = 0, y = -10 }, -- Up
		[tostring(Key.Down)] = { x = 0, y = 10 }, -- Down
		[tostring(Key.Left)] = { x = -10, y = 0 }, -- Left
		[tostring(Key.Right)] = { x = 10, y = 0 }, -- Right
	}

	local delta = deltamap[tostring(e.key)]

	print("Modifiers: " .. (e.modifiers.alt and "Alt " or "") .. (e.modifiers.shift and "Shift " or "") .. (e.modifiers.ctrl and "Ctrl " or "") .. (e.modifiers.logo and "Logo " or ""))

	if e.modifiers.alt and delta then
		e:prevent_default()
		print("Moving output")
		local output = wm.outputs:name("winit")
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
	-- print("Pointer moved to " .. e.position.x .. ", " .. e.position.y .. " with delta " .. e.delta.x .. ", " .. e.delta.y)

	-- lets try adding output dragging with mb1 + mouse move

	if e.pointer:buttons() then
		print("Pointer buttons: " .. table.concat(e.pointer:buttons()))

		if e.pointer:buttons()[1] then
			print("Dragging output")
			local output = wm.outputs:name("winit")
			if not output then
				print("Output not found")
				return
			end
			local pos = output:position()
			print("Current position: " .. pos.x .. ", " .. pos.y)
			output:set_position({
				x = pos.x - e.delta.x,
				y = pos.y - e.delta.y
			})
		end
	end
end)

wm.input:on("pointer_button", function(e)
	
end)

-- wm.outputs.winit:move({ x = -200, y = -200 })
