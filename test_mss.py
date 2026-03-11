import mss
try:
    with mss.mss() as sct:
        print("Monitors:", sct.monitors)
        sct.grab(sct.monitors[0])
        print("Grabbed 0 successfully")
        sct.grab(sct.monitors[1])
        print("Grabbed 1 successfully")
except Exception as e:
    print("Error:", e)
