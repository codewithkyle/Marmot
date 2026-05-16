local regular_price = data.getSlot("regular_price") or 0

if regular_price < 5.0 then
    frame.visible = false
end
