local promo_url = data.getSlot("promo_url") or ""

if promo_url == "" then
    frame.visible = false
end
