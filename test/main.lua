print("Meowing on " .. wm.name)

wm.input:on("event", function (data)
	print("MEOW EVENT")
	wm:doohickey()
end)
