
from qibullet import SimulationManager

simulation_manager = SimulationManager()
client_id = simulation_manager.launchSimulation(gui=True, use_shared_memory=True, auto_step=False)

nao = simulation_manager.spawnNao(
        client_id,
        spawn_ground_plane=True)

try:
    while True:
        # Your code here...
        simulation_manager.stepSimulation(client_id)

except KeyboardInterrupt:
    pass

simulation_manager.stopSimulation(client_id)