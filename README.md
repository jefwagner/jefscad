# Jefscad

**What is Jefscad**

A solid modeling language based on constructive solid geometry. Solids are created by 
writing code, inspired by Openscad, but with a python front-end so we can use the full
power of python scripting/language when creating solids.

**Why create somthing new? Why not use openscad?**

Openscad is a very good program, and I recommend it's use to many others. However I have
run into three problems that I would like to address 

1. Openscad uses floating point numbers, and taking unions or differences can lead to
objects that should connect not connecting or objects that should subtract prefectly 
leaving an extremely thin left-over part. This can be fixed by adding small offsets when 
creating objects, but it would be nicer if we wouldn't have to do that.

2. Openscad creates meshes at object creation. So a cylinder is actually an extruded
polygon. I would like to keep objects as abstract shapes until it is time to render or
export, and only then create the mesh.

3. Openscad does not create or read .step files to sharing with other solid modeling
programs or CNC services.

So, as a personal project, I want to create my own solid modeling language.

## How do we address these three issues?

We will try and architect the solid modeling system from the start to try and address
the three pain points of using Openscad.

### Dealing with floating point numbers:

To address the first issue, we will use rounded floating point intervals, which I've
called `Flint`s instead of floating point numbers. The idea behind a rounded floating
point interval is that we know that floating point numbers are inexact, so we keep a
lower and upper bound for what the exact value should be, and after every mathematic
acttions (adding, multiplying, applying functions) we grow the interval slightly to
guarantee that the result is still within the interval. We can now define comparison
operators for the `Flint` objects such that they will compare as equal if the intervals
overlap at all. This addresses a classic issue with floating point number things that
should be equal are not (this should address the issues with merging or subtracting
solids), but it does introduce the case where equality is not transitive. You win some,
you lose some I guess.

### Dealing with early meshing:

To address the second issue, we will structure the creation of solids in three steps:
1. We only keep a construct solid geometry (csg) the captures the initial shapes and
   operations. This is still exact at this point.
2. We transform the csg solids into boundrary representations, where the face surfaces
   can be, represented by generic functions so we still keep exact representation. We
   will preferentially use NURBS surfaces since they can exactly represent circular or
   spherical structures, and simple tranformatiosn (precisly affine transformations) of
   the surface is easily obtained by transforming the control points.
3. The boundary representations can then be meshed using something like Chew's second
   algorithm to get a mesh of whatever resolution we need when we need to render the
   solids or export them for 3-D printing.

The idea is that we will mostly use steps 1 and 2 during the object creation or
manipulation, and in both of those the shapes are 'exact' (i.e. circle are actual
circles and not N-sided regular polygons).

### Sharing files:

This is the hardest issue. I don't know exactly what the .step file format looks like.
It is an accepted standard, but you have to purchase it's specification to get the full
details. From what I have been able to find online, I _think_ that the .step files can
represent solids using the boundary representation. If that's the case, there is a good
chance that we will be able to support exporting solids into .step files, and we _might_
be able to support importing solids (not sure on this one - cause we would miss the
step 1 representation mentioned above).

