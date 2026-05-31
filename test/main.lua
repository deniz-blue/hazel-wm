print("Meowing on " .. wm.name)

-- wm.input.keyboard:set_layout("tr")

wm:on("ready", function()
	print("WM is ready!")
	spawn("alacritty")
end)

wm.input:on("key", function(e)
	print("KEY EVENT: " .. e.keycode)

	-- Alt + T
	if e.modifiers.alt and e.keycode == 28 then
		print("Alt + T pressed")
		spawn("alacritty")
		e:prevent_default()
	end

	-- Alt + R
	if e.modifiers.alt and e.keycode == 27 then
		wm.outputs:name("winit"):set_position({ x = 0, y = 0 })
		e:prevent_default()
	end

	local delta = {
		[111] = { x = 0, y = -10 }, -- Up
		[116] = { x = 0, y = 10 }, -- Down
		[113] = { x = -10, y = 0 }, -- Left
		[114] = { x = 10, y = 0 }, -- Right
	}

	if e.state == "Pressed"
		and e.modifiers.alt
		and delta[e.keycode] then
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
			x = pos.x + delta[e.keycode].x,
			y = pos.y + delta[e.keycode].y
		})
	end
end)

wm.input:on("pointer_move", function(e)
	
end)

wm.input:on("pointer_button", function(e)
	
end)

wm.outputs:on("added", function(e)
	print("Output added " .. wm.outputs:count())
end)

-- wm.outputs.winit:move({ x = -200, y = -200 })
