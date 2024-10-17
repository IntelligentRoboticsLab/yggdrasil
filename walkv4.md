# walkv4

## goals

The goal of walkv4 is to improve the walking engine of the robot,
prioritizing the following:
- stability
- speed
- efficiency

additionally the motion architecture should be simplified and modularized.
the current walkv3 is stateful and a side-effects galore.

for example, walkv3 has a constant step duration of `250 ms`, which makes it
difficult to adjust the planned gait when balancing. walkv4 should be able to
adjust the step duration on the fly.

## design

The design of walkv4 aims to remove the side-effects and make the
motion engine more modular. The walking engine should be able to
be run in a loop, and the state should be passed in and out of the
engine.

the following sections describe the planned improvements for walkv4.

### step

there should be a concept of step duration, which can be adjusted
on the fly. to achieve this the steps should be planned in advance.

a step is defined by the following properties:
- forward distance
- lateral distance
- turn angle
- duration
- height at the apex
- swing side

using these properties, the engine uses the parameters to create trajectories for
the feet.
