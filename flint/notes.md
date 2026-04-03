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
[x] Implement unit tests for formatting for array types
[x] Implement formatting for array types
    Format: single line, square brackets, comma-space separated (e.g. `[1, 2.51, 3.14]`)
[x] Implement Display for FlintMut and FlintViewMut
[_] Implement pretty-printing for 4-element (4x1 column vector) and 16-element (4x4
    matrix) FlintArray/FlintVec/FlintView types with newlines and aligned columns

### Conversions

I want to be able to simply create Flint's from every standard numeric type, and have
macros for creating array-types.

[x] Implement unit tests for converting from standard numeric types to scalar Flints
[x] Implement `From` or `Into` traits for standard numeric types
[x] Implement unit tests for converting multiple values into array Flints
[x] Implement macros that create array Flints from arbitrary numeric types

### Comparisons

We will define equal as intervals overlap, and less than greater than for when the
intervals lie completely on on-side or the other (with NANs never equal). I'm not sure
what the equivalent ops should be fore the array types, since they would naturally
return an array of bools instead of a single bool.

[x] Implement unit tests for comparisons with scalar Flint types
[x] Implement scalar comparisons
[x] Implement comparisons for FlintMut
[x] Figure out and document the appropriate equivalent comparisons for array types
    Decision: do NOT implement PartialEq/PartialOrd for array types — the return type
    would need to be an array of bools, which the standard traits cannot express.
    Instead, provide named methods on each array type:
      - eq_intervals / lt_intervals / gt_intervals -> [bool;N] or Vec<bool>
      - all_eq / all_lt / all_gt -> bool (aggregates)
    FlintArray uses SIMD (portable_simd) for the element-wise methods.
    FlintVec and FlintView use chunked SIMD (lane=8) with scalar fallback.
[x] Implement unit tests for array comparisons
[x] Implement array comparisons
[x] Implement array comparisons for FlintViewMut

### Arithmatic

We would like to implement the arithmatic operators: unary `-`, `+`, `+=`, `-`, `-=`,
`*`, `*=`, `/`, `/=`. The 'other' for these are any types whose scalar compoments 
implement `Into` flint. The output for the non-assignments are always an owned type,
but the assignment operators can reassign the ref-types.

[x] implement unit tests for scalar, and matching size array arithmatic
[x] Implement scalar and matching size array arithmatic
    Scalar (Flint, FlintRef): Neg, Add, Sub, Mul, Div + *Assign ops; Rhs generic over Into<Flint<T>>.
    Arrays (FlintArray, FlintVec, FlintView): element-wise SIMD; chunked f32x8/f64x8 for Vec/View
    with scalar remainder; 4-boundary min/max for Mul/Div; FlintView ops return FlintVec (owned).
[x] Implement arithmatic for FlintMut and FlintViewMut

### Broadcasting

Support operations between scalars and arrays by broadcasting the scalar to a uniform
array of the same size. There are some questions of how to support broadcasting between
arrays of different sizes. I doubt it is possible without the full size (m,n), but we 
maybe want to add some methods that allow operations between 1x4, 4x4, and 4x1 matrices 
since they are likely common operations when working in 3D with homogenous coordinates.

[x] Figure out and document appropriate broadcasting between for scalar->array and
    non-matching array size.
    - for array <op> scalar and scalar <op> array: treat the scalar as an equal sized 
      array with uniform elements.
    - for operations between 16-length arrays and 4-length arrays -> make explicit
      named methods on the 16 length array:
      - row_wise_<op>: broadcast the 4-length [a,b,c,d] ->
        [a,a,a,a,b,b,b,b,c,c,c,c,d,d,d,d] (uniform rows in row major storage)
      - col_wise_<op>: broadcast the 4-length [a,b,c,d] ->
        [a,b,c,d,a,b,c,d,a,b,c,d,a,b,c,d] (uniform cols in row major storage)
[x] Implement unit tests for broadcasting arithmatic
[x] Implement broadcasting arithmatic
    - From<Flint<T>> for FlintArray<T,N> (splat) enables array op scalar via existing Rhs: Into<> bound
    - impl_scalar_array_arith! macro: Flint<T> op FlintArray<T,N> for all 4 ops (both types)
    - FlintVec + Flint<T> and Flint<T> + FlintVec: all 4 ops + *Assign, chunked SIMD (lane=8) with splat
    - FlintView + Flint<T> and Flint<T> + FlintView: all 4 ops (no assign — views immutable)
    - FlintViewMut: non-assign delegates to FlintView; assign ops with Simd::splat in-place
    - impl_row_col_wise! macro: row_wise_*/col_wise_* named methods on FlintArray<T,16>
      row-wise: rhs[i/4], col-wise: rhs[i%4]; all 4 ops; SIMD<T,16> for both f32 and f64

### Basic linear algebra

The 4-length arrays will be thought of a 1x4 column vectors and the 16-length arrays
will be thought of as 4x4 matrices stored in C-style row major format. I want to support
the following linear algebra operations
* matrix-matrix multiplication for two 16-length arrays (method with name mat_mul)
* matrix-vector multiplication for 16-length and 4-length (method with name dot? -> not
  sure on name here. I don't like overloading method names, but dot seems the most
  natural)
* vector-vector dot product for 4-length and 4-length (method with name dot)
* matrix-[array-of-vectors] -> [array-of-vectors]. In this case I want to support
  matrix multiplication for a larger 4N-length array and we do matrix vector 
  multiplication (in matrix notation, if A is 4x4 and B is 4xN, I want
  A-matmul-Transpose(B))
* vector-[array-of-vectors] -> [array-of-scalars]. Similar to above - apply a dot
  product to a 4N-length array (in matrix notation if A is 1x4 and B is 4xN, I want
  Transpose(A)-dot-Transpose(B))
* det3 -> take the 3x3 determinant of the upper left 3x3 submatrix of a 4x4 matrix
* det4 -> take the 4x4 determinant
* svd3 -> do a singular value decomp of the upper left 3x3 submatrix of a 4x4 matrix
  with some conditions so that it represents rotation-scale-rotation operations
  - both rotations matrices should have determinant positive 1
  - the singular values should be sorted to be unique as possible (largest to smallest
    absolute value? if one must be negative, make it the last one? - dunno)

[x] Implement unit tests for simple mat-mat, mat-vec, and vec-vec products
[x] Implement lin-alg methods for mat_mul, the mat-vec product, and dot
[x] Implement unit tests for matrix array-of-vectors and vector array-of-vectors
[x] Implement lin-alg methods for array-of-vectors products
    apply_batch / dot_batch: vectors stored as rows in a flat 4N SoA (Nx4 layout).
[x] Implement unit tests for determinants
[x] Implement methods for determinants
    det3: upper-left 3x3 submatrix; det4: full 4x4 cofactor expansion along row 0.
[_] Document the choices to try and make the svd unique
[_] Implement unit tests for svd
[_] Implement method for svd
    Note: svd3 deferred — it's a convenience method (simplify transforms into
    rotate-scale-rotate-translate) and not needed for authoring or evaluation.

### Standard math function

Implement all the standard math functions defined on floating point numbers for scalar
and array types, with broadcasting as appropriate for two-input functions.

[_] implement unit tests
[_] implement math functions

