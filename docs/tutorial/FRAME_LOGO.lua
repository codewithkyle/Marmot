local regular_price = data.getSlot("regular_price") or 0
local get_qty = data.getSlot("get_qty") or 0
local savings = regular_price * get_qty
local promo_url = data.getSlot("promo_url") or ""

if promo_url ~= "" then
    frame.visible = false
    return
end

if savings >= 25.0 then
    frame.value = "logo_alt"
else
    frame.value = "logo_default"
end
