
from qibullet import SimulationManager
import numpy as np
import pybullet as p
import signal
import sys

simulation_manager = None
client_id = None
nao = None
angle_text = None

def main():
    simulation_manager = SimulationManager()
    client_id = simulation_manager.launchSimulation(gui=False, use_shared_memory=True)
    nao = simulation_manager.spawnNao(
            client_id,
            spawn_ground_plane=False)

    angle_text = p.addUserDebugText("angle: 0", [0.1, 0.15, 0.1], [0, 0, 0], 1)


    for i in range(100000):
        try:
            angle = ((0.5 * np.sin(0.03 * i) + 0.5) * -np.pi/2) + np.pi/4
            # angle = -1.1155553380155812
            angle_text = p.addUserDebugText(f"angle: {angle}", [0.1, 0.15, 0.1], [0, 0, 0], 1, replaceItemUniqueId=angle_text)
            nao.setAngles(["LHipYawPitch"], [angle], [1])
            simulation_manager.stepSimulation(client_id)
        except KeyboardInterrupt:
            signal.signal(signal.SIGINT, handler)
            simulation_manager.removeNao(nao)
            p.removeUserDebugItem(angle_text)
            print("press ctrl+c to exit!")
            sys.exit()


def handler(signal, frame):
    print('exit')
    sys.exit(0)

if __name__ == "__main__":
    main()