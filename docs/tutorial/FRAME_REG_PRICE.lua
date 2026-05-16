local regular_price = data.getSlot("regular_price") or 0
frame.value = concat("Reg ", currency(regular_price))
