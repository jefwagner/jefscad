# Planar SSI test cases

Consider axis-alligned cuboids A and B

lets number the faces of A
1 -> top (+z)
2 -> botton (z=0)
3 -> right (+x)
4 -> left (x=0)
5 -> back (+y)
6 -> front (y=0)

lets number the faces of B
7 -> top (+z)
8 -> botton (z=0)
9 -> right (+x)
10 -> left (x=0)
11 -> back (+y)
12 -> front (y=0)

## Test case 1: parallel faces, different planes
let both A and B be unit cubes, and move B up (+z) by 0.1 units
parallel different plane non-intersecting faces are faces 1 and 7

## Test case 2: parallel faces, same planes, not intersecting
let both A and B be unit cubes, and move B right (+x) by 1.1 units
parallel same plane non-intersecting faces are faces 1 and 7

## Defered test case: paralle faces, same plan, coincident faces
let both A and B be unit cubes, and move B right (+x) by 0.5 and back (+y) 0.5 units
parallel same plane coincident faces are faces 1 and faces 7

## Test case 3: non-parallel, non-intersecting faces
let both A and B be unit cubes, and move B up (+z) 0.5 and right (+x) by 1.1 units
non-parallel, non-intersecting faces faces are faces 1 and 10

## Test case 3b: non-parallel, non-intersecting, t-intervals disjoint
Let A and B be unit cubes.
Move B up (+z) 0.5, right (+x) 0.5, and back (+y) 1.5 units.
B occupies x∈[0.5,1.5], y∈[1.5,2.5], z∈[0.5,1.5].
The faces in question are face 1 (A top: z=1, x∈[0,1], y∈[0,1]) and face 10 (B left: x=0.5, y∈[1.5,2.5], z∈[0.5,1.5]).
The plane-plane intersection line is x=0.5, z=1, direction along +y.
clip_line_to_face on face 1 returns t∈[0,1] (y-range of face 1).
clip_line_to_face on face 10 returns t∈[1.5,2.5] (y-range of face 10).
The t-intervals are disjoint: no intersection.
This is distinct from test case 3 — both clips succeed, but the result is still None.

## Test case 4: non-parallel, intersect one face fully internal to other
let A be a unit cube
let B be a cuboid with (x-width, y-depth, z-height) = (1, 0.5, 1)
move B up (+z) 0.5 and right (+x) 0.5, and back (+y) 0.1 units
non-parallel, intersecting, fully internal faces are 1 and 10
two edges of face 10 on B cross face 1 on A

## Test case 5: non-parallel, intersecting one point internal, one edge-intersection
let A be a unit cube
let B be a cuboid with (x-width, y-depth, z-height) = (1, 0.5, 1)
move B up (+z) 0.5 and right (+x) 0.5
non-parallel, insecting face 1 on A and face 10 on B
one edge of face 1 of A and one edge of face B cross
one edge of face 10 on B crosses goes through the interior of face 1 on A

## Test case 6: non-parallel, intersecting two edge-intersections
Let A and B be unit cubes
Move B up (+z) 0.5 and right (+x) 0.5
non-parallel, intersecting faces are 1 and 10
two edges of face 1 on A intersect two edges from face 10 on B

## Test case 7: non-parallel, intersecting edge-loops-link
Let A and B be unit cubes
Move B up (+z) 0.5, and right (+x) 0.5 and back (+y) 0.5 unites
non-parallel, intersecting face 1 on A and face 10 on B
one edge of face 1 on A intersects in the interior of face 10 on B
one edge of face 10 on B intersects in the interior of face 1 on A

