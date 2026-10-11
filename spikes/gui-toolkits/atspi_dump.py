#!/usr/bin/env python3
"""Dump the AT-SPI tree of an app by name. Usage: atspi_dump.py APPNAME_SUBSTRING"""
import sys, gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi
want = sys.argv[1].lower()
desk = Atspi.get_desktop(0)
def walk(n, d=0):
    try:
        st = n.get_state_set()
        names = [s.value_nick for s in (Atspi.StateType(i) for i in range(0, 45)) if st.contains(s)] if d < 0 else []
        print("  " * d + f"[{n.get_role_name()}] {n.get_name()!r}" + (f" text={Atspi.Text.get_text(n.get_text_iface(), 0, 80)!r}" if n.get_text_iface() else ""))
        for i in range(n.get_child_count()):
            walk(n.get_child_at_index(i), d + 1)
    except Exception as e:
        print("  " * d + f"(err {e})")
found = False
for i in range(desk.get_child_count()):
    app = desk.get_child_at_index(i)
    if app and want in (app.get_name() or "").lower():
        found = True; walk(app)
print("apps seen:", [desk.get_child_at_index(i).get_name() for i in range(desk.get_child_count())]) if not found else None
