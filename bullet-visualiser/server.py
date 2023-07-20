import sys
from qibullet import SimulationManager
import numpy as np
import pybullet as p

if __name__ == "__main__":
    simulation_manager = SimulationManager()
    client_id = simulation_manager.launchSimulation(gui=True)

    while True:
        simulation_manager.stepSimulation(client_id)
        pass
        
