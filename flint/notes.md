# Implementation notes and ToDo.

The basic `Flint` struct is a simple struct with a lower bound and upper bound

```rust
struct Flint<T>{ lb: T, ub: T };
```

I would like to define 5 base types, all backed by 32bit and 64 bit floats (10 types
total)

* `Flint` An owned type that implements the `Copy` trait.
* `FlintRef` A reference type where the lower and upper bounds are references instead of 
  owned types.
* `FlintArray` An owned fixed size array type where the lower and upper bounds are owned 
  fixed sized arrays of floats. In particular, 4-size (for 3-D vectors in homogenous 
  coordinates) and 16-size (for 4x4 affine transformation matricies) will be 
  particularly useful. I suspect that the 4-size array should also implement the COPY 
  trait, and maybe all fixed sized array backed types should.
* `FlintVec` An owned type where the lower and upper bounds are heap-allocated vectors.
* `FlintView` A reference type where the lower and upper bounds are slices borrowed from
  either a FlintArray or FlintVec.

Decision: No shape/stride metadata on any type. Multi-dimensional semantics (e.g. 4×4
matrix) are encoded in the const generic N and in the methods defined on top of it.
This keeps FlintArray Copy, layout simple, and SIMD straightforward.

In addition, I would like to implement
* Formatting to print out the shortest number possible in the range
* Conversions to and from standard numeric types
* Comparison operators for the scalars and appropriate equivalents for the array types
* Arithmatic operators for scalars and array types
* Broadcasting between scalar *op* array or array *op* array on non-identical sizes
* Basic linear algebra for 4x4 matrices
* Standard math functions for scalar and array types

Where possible, I would like to take advantage of the SIMD instructions for array types.

## ToDo:

### Formatting

When printing `Flint` values, I would like to treat them as the most rounded value
within the interval. Debug printing can be derived, but the standard print will need a
custom implementation. I use the `ryu` crate because I like it's formatting of large and
small values more than the rust standard libraries. I'm also not sure what the
formatting for the array types should be: all one-line or should we introduce newlines
and spacing to have them look better when printted to the command line?

[x] Implement unit tests for formatting scalar types
[x] Implement formatting for scalar types
[x] validate formatting in edge case where upper bound is very near but strictly less
    than a much shorter representation when printed in 17 digit form 
    (ex. 1.59999999999999994 vs 1.6)
    Note: the actual bug was for negative intervals — truncating the 17dp fractional
    string on a negative number moves the value toward zero (above ub), so the original
    `lb <= ub_trunc` check was insufficient. Fixed by adding `&& ub_trunc <= ub`.
[_] Implement unit tests for formatting for array types
[_] Implement formatting for array types

### Conversions

I want to be able to simply create Flint's from every standard numeric type, and have
macros for creating array-types.

[x] Implement unit tests for converting from standard numeric types to scalar Flints
[x] Implement `From` or `Into` traits for standard numeric types
[_] Implement unit tests for converting multiple values into array Flints
[_] Implement macros that create array Flints from arbitrary numeric types

### Comparisons

We will define equal as intervals overlap, and less than greater than for when the
intervals lie completely on on-side or the other (with NANs never equal). I'm not sure
what the equivalent ops should be fore the array types, since they would naturally
return an array of bools instead of a single bool.

[x] Implement unit tests for comparisons with scalar Flint types
[x] Implement scalar comparisons
[_] Figure out and document the appropriate equivalent comparisons for array types
[_] Implement unit tests for array comparisons
[_] Implement array comparisons

### Arithmatic

We would like to implement the arithmatic operators: unary `-`, `+`, `+=`, `-`, `-=`,
`*`, `*=`, `/`, `/=`. The 'other' for these are any types whose scalar compoments 
implement `Into` flint. The output for the non-assignments are always an owned type,
but the assignment operators can reassign the ref-types.

[_] implement unit tests for scalar, and matching size array arithmatic
[_] Implement scalar and matching size array arithmatic

### Broadcasting

Support operations between scalars and arrays by broadcasting the scalar to a uniform
array of the same size. There are some questions of how to support broadcasting between
arrays of different sizes. I doubt it is possible without the full size (m,n), but we 
maybe want to add some methods that allow operations between 1x4, 4x4, and 4x1 matrices 
since they are likely common operations when working in 3D with homogenous coordinates.

[_] Figure out and document appropriate broadcasting between for scalar->array and
    non-matching array size.
[_] Implement unit tests for broadcasting arithmatic
[_] Implement broadcasting arithmatic

### Basic linear algebra

I would like to implement simple vector for 4x4 matrices including: matrix-matrix and
matrix-vector multiplication, and singular value decomposition.

[_] Implement unit tests
[_] Implement vector math

### Standard math function

Implement all the standard math functions defined on floating point numbers for scalar
and array types, with broadcasting as appropriate for two-input functions.

[_] implement unit tests
[_] implement math functions

