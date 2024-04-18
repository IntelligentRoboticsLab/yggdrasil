# Behaviors (in order of match appearance)

## StartUp (constant) x
Keep the robot standing up right if the process was closed

## Unstiff (constant) x
Make the robot sit down and then release all joints

## Initial (robot dependent)
Stand up right, stationairy, whilst looking at the Middle circle

## Ready (robot(/role) dependent)
Walk to designated place on the field.
Each location differers, but depends on the robots number.

## Set (robot dependent)
Look at the middle circle, unless a ball has been spotted.

## Playing (role dependent)
Depending on the role assigned to the robot we will have different behavior trees. This way we can have robots switch roles, through communication. This switching might accors during the regonition of a ball or if the starting keeper is penalized.

## Penalized
It's immportant to correctly handle returning from penalized as we could be on either side of the field.

## Finished (constant)
Sit down the robot if this is reliable
Otherwise stad still

# New behaviors to add

## StandingLookAt

## WalkTo

