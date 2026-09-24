#!/bin/sh
udevadm control --reload-rules || true
udevadm trigger --action=change --subsystem-match=wmi --sysname-match=7A4DDFE7-5B5D-40B4-8595-4408E0CC7F56 || true
