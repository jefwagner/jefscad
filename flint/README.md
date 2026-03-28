# Rounded Floating Point Intervals

This crate defines and implement a family of rounded floating point interval types

*`Flint<f32>` and `Flint<f64>` - for owned simple types
*`FlintRef<'a, f32>` and `FlintRef<'a, f64>` - with references instead of owned types
*`FlintArray<N,f32>` and `FlintArray<N,f64>` - A fixed sized owned array
*`FlintVec<f32>` and `FlintVec<f64>` - A heap allocated owned array
*`FlintView<'a, f32>` and `FlintView<'a, f64>` - A view into a vec or array

Note: The proliferation of types is due to trying to make using flints more performant 
by taking advantage of SIMD instructions, while using the struct-of-arrays vs
array-of-struct formats.

## What is a rounded floating point interval?

A rounded FLoating point INTerval (`Flint`) is a data-structure that attempts to 
address some of the shortcomings of using floating point numbers in places where we want
to do exact comparisons.

### The problem with floating point numbers

A floating point numbers is similar to a number written in decimal with a decimal point
and a finite number of digits. It is well known some numbers have infinite decimal
representations, for example the fraction 1/3 with the decimal representation 0.3333…
repeating to infinity. But a floating point numpy can only includes a finite number of
digits, so we have to truncate the number, and it will no longer exactly represent 1/3.
Here-in lies the first issue we can have with floating point numbers:

**Floating pointer numbers can not represent all numbers exactly.**

For many very important numbers such as or or even the humble 1/3rd do not have finite
decimal representations, and so they can not be represented exactly by floating point
numbers. Frequently, we can not exactly capture the input needed for some mathematical
calculations. For example, since we can not exactly represent with floating point
numbers the result of , where represents the ‘nearest floating point’ to , will be close
to, but NOT equal zero.

This limited ability to represent numbers leads to another consequence of floating point
numbers:

**Exact math with floating point numbers does not always yield floating point numbers.**

A very simple example is just division. The number 1 is exactly representable, as is the
number 3. But the result of dividing 1 by 3, the fraction 1/3rd is NOT representable.
This result can lead to some results that seem to invalidate math. A classic example
with binary floating point numbers is this: 2/10 + 2/10 + 2/10 will not yield the same
result as (2+2+2)/10. Go ahead and try it out. Open up python and try

```python
(0.2 + 0.2 + 0.2) == 0.6
```

you will find that the result yields False. This problem is well known, and most
experienced programmers have a rule to never check equality with floating point numbers.
When an equality comparison is required, a ‘close enough’ style comparison of the type
where the absolute different is required to be less than some predetermined small value
epsilon. An example python implementation could be

```python
almost_eq(a: float, b: float, eps: float = 1.0e-8):
    """Compare two floats to see if they are close to each other"""
    return abs(b-a) < eps
```

This works well enough if we know the approximate size of the numbers we expect to be 
working with, but will often fail if we are working with large ( > 1 billion) or small 
( < 1 billionth) numbers and both of those values are WELL within the bounds of what 
can be represented by the typical 64 bit floating point number.

### Introducting `Flint`s

Let us introduce a new type of number that addressed the two issues above: the rounded
floating point interval or flint. To fully understand how these numbers (or really
data-structures) allow us to address some of the issues of floating point numbers, lets
break down the name in reverse order. First, notice the ‘interval’ in the name. Unlike
typical numbers, which can be represented by 0-length point on a number-line, a flint
will be represented by a small but finite-length interval with an upper and lower bound.
The ‘exact’ value of any number can now be captured as long as it lies between the upper
and lower bound.

A very real objection to the new number might be: it’s and interval; it’s not really a
number! This is true, but I as long as the interval is small it CAN be treated as a
number and the size of the interval can capture the uncertainty in the exact value. That
brings us to the second term in the name ‘floating point’. In a flint, the upper and
lower bounds of the interval are floating point numbers. Remember, that a floating point
number is number with a decimal point and a finite number of non-zero digits. An
important concept relating to those finite number of digits for floating point numbers
is the ‘unit in last place’ or ulp. One ulp is the distance between two consecutive
floating point number with a difference of 1 in the least significant digit. Now, when
we want to represent any number $x$, we can turn that number into it’s nearest floating
point $\text{nfp}(x)$, and then define the upper and lower bounds as one ulp above and
below $\text{nfp}(x)$. For a 64 bit number, the unit in last place is typically 16
orders of magnitude smaller than the number itself, so this new number is still quite
precise.

Now you are perhaps satisfied that the interval can represent a number, AND we can make
sure that any exact number can be captured in a small interval of only a few ulp wide.
Lets try and satisfy the last criterion: can we guarantee that we can calculate a new
small interval for all math operations that is guaranteed to hold the exact result from
the all numbers contained in the input intervals? Yes, and we do so with the first and
final term in the name, by ‘rounding’ the interval after each math operation. For all
continuous functions, an interval in the input will map to an interval in the result,
with the endpoints of the interval OR the extrema of the function mapping to the
endpoints of the resulting interval. The IEEE-754 standard for floating point numbers
requires that the result of all math operations be within 1 ulp of the exact result for
exact inputs. This means if we round the lower boundary down by 1 ulp and round the
upper boundary up by 1 ulp we can guarantee that the resulting interval will contain the
exact result of all possible values in the input interval. This can grow the interval as
more and more operations are performed, but this can be a be thought of as capturing the
growing uncertainty of the final result from using floating point numbers in the first
place.

## References

The following references are use for the description of the floating point interval

[Patrikalakis et al](https://web.mit.edu/hyperbook/Patrikalakis-Maekawa-Cho/node46.html) 
contains details of the mathematical implementation of the flint objects.


