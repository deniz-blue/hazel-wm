print("Meowing on " .. wm.name)

-- wm.input:on("event", function (data)
-- 	wm:doohickey()
-- end)

wm:on("commit", function ()
	print("COMMIT EVENT")
end)
