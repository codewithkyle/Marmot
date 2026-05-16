local promo_url = trim(default(data.getSlot("promo_url"), ""))

frame.visible = promo_url ~= ""
frame.value = frame.visible and promo_url or nil
