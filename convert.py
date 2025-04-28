import json

# Your input dictionary
data = {
    "head_yaw": 0.0,
    "head_pitch": 0.3839724354387525,
    "left_shoulder_pitch": 1.918992042541504,
    "left_shoulder_roll": 0.12421202659606934,
    "left_elbow_yaw": 0.20244598388671875,
    "left_elbow_roll": -1.2578380107879639,
    "left_wrist_yaw": -0.052197933197021484,
    "left_hand": 0.0,
    "right_shoulder_pitch": 1.9620280265808105,
    "right_shoulder_roll": -0.26695799827575684,
    "right_elbow_yaw": -0.14883995056152344,
    "right_elbow_roll": 1.376039981842041,
    "right_wrist_yaw": 0.056715965270996094,
    "right_hand": 0.0,
    "left_hip_yaw_pitch": 0.0,
    "left_hip_roll": 0.0,
    "left_hip_pitch": 0.0,
    "left_knee_pitch": 1.9198621771937625,
    "left_ankle_pitch": 0.0,
    "left_ankle_roll": 0.0,
    "right_hip_yaw_pitch": 0.0,
    "right_hip_roll": 0.0,
    "right_hip_pitch": 0.0,
    "right_knee_pitch": 1.9198621771937625,
    "right_ankle_pitch": 0.0,
    "right_ankle_roll": 0.0,
}

# Mapping for sections
sections = {
    "head": {},
    "arms": {"left_arm": {}, "right_arm": {}},
    "legs": {"left_leg": {}, "right_leg": {}},
}

# Distribute values into correct sections
for key, value in data.items():
    if key.startswith("head_"):
        part = key.split("_")[1]
        sections["head"][part] = value
    elif (
        key.startswith("left_")
        and "shoulder" in key
        or "elbow" in key
        or "wrist" in key
        or "hand" in key
    ):
        part = "_".join(key.split("_")[1:])
        sections["arms"]["left_arm"][part] = value
    elif (
        key.startswith("right_")
        and "shoulder" in key
        or "elbow" in key
        or "wrist" in key
        or "hand" in key
    ):
        part = "_".join(key.split("_")[1:])
        sections["arms"]["right_arm"][part] = value
    elif key.startswith("left_") and ("hip" in key or "knee" in key or "ankle" in key):
        part = "_".join(key.split("_")[1:])
        sections["legs"]["left_leg"][part] = value
    elif key.startswith("right_") and ("hip" in key or "knee" in key or "ankle" in key):
        part = "_".join(key.split("_")[1:])
        sections["legs"]["right_leg"][part] = value


# Function to print in TOML style
def print_toml(sections):
    print("[motions.angles.head]")
    for k, v in sections["head"].items():
        print(f"{k} = {v}")
    print()

    print("[motions.angles.arms.left_arm]")
    for k, v in sections["arms"]["left_arm"].items():
        print(f"{k} = {v}")
    print()

    print("[motions.angles.arms.right_arm]")
    for k, v in sections["arms"]["right_arm"].items():
        print(f"{k} = {v}")
    print()

    print("[motions.angles.legs.left_leg]")
    for k, v in sections["legs"]["left_leg"].items():
        print(f"{k} = {v}")
    print()

    print("[motions.angles.legs.right_leg]")
    for k, v in sections["legs"]["right_leg"].items():
        print(f"{k} = {v}")
    print()


# Run it
print_toml(sections)
