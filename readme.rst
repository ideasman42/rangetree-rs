
Range Tree
==========

Data type for storing non-overlapping scalar ranges.
The underlying representation is a red-black tree for fast manipulation.


Usage
-----

This may be used for tracking free ID's or ranges.

Ranges are typically integers however generic types are used,
so any type with a ``One`` and ``Zero`` trait (along with addition and subtraction support)
may be used.


Further Work
------------

While the API is complete on a basic level,
there are some additions that could be useful.

- Range Queries: to check if a value within a range is taken.
- Boolean Operations: support for performing binary operations on range-trees (and, or, xor, invert).
- Set Operations: is-subset, is-superset, is-disjoint.
- Interval Iterator: to loop over used or unused intervals.


License
-------

Apache 2.0, see ``LICENSE`` file.
