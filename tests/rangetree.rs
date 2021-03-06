// Apache License, Version 2.0
// (c) Campbell Barton, 2016

extern crate rangetree;

use rangetree::RangeTree;

#[test]
fn test_basic_take_release() {
    let mut r: RangeTree<i32> = RangeTree::new([0, 10], false);

    let i = r.take_any().unwrap();
    assert_eq!(i, 0);
    assert!(!r.has(i));

    let i = r.take_any().unwrap();
    assert_eq!(i, 1);
    r.release(0);

    let i = r.take_any().unwrap();
    assert_eq!(i, 0);
    assert!(!r.has(i));
}

#[test]
fn test_take_all() {
    let mut r: RangeTree<u8> = RangeTree::new([0, 255], false);
    assert!(r.is_empty());
    for i in 0..255 {
        assert_eq!(i, r.take_any().unwrap());
    }
    assert_eq!(255, r.take_any().unwrap());
    assert!(!r.is_empty());

    for i in 0..255 {
        r.release(i);
    }

    // take all again
    for i in 0..255 {
        assert_eq!(i, r.take_any().unwrap());
    }
    // leave 255 in
    for i in 0..255 {
        r.release(i);
    }
}

#[test]
fn test_retake() {
    let mut r: RangeTree<u8> = RangeTree::new([0, 32], false);
    for i in 0..16 {
        r.take(i * 2);
    }

    let mut n: usize = 0;
    for i in 0..32 {
        n += if r.retake(i) { 1 } else { 0 };
    }
    assert!(n == 16);
    for i in 0..16 {
        r.release((i * 2) + 1);
    }

    for i in 0..16 {
        assert!(r.has((i * 2) + 1));
        assert!(!r.has(i * 2));
    }

    // println!("{:?}", r.ranges_as_vec());
}

#[test]
fn test_complex() {
    let mut r: RangeTree<i32> = RangeTree::new([-10, 11], false);
    for _ in 0..2 {
        assert!(r.is_empty());
        for i in &[-10, 10, 11] {
            r.take(*i);
        }
        assert_eq!(r.ranges_taken_as_vec().as_slice(),
                   [[-10_i32, -10], [10, 11]]);

        for i in &[-8, -7, 8] {
            r.take(*i);
        }
        assert_eq!(r.ranges_taken_as_vec().as_slice(),
                   [[-10, -10], [-8, -7], [8, 8], [10, 11]]);

        for i in &[-9, 9] {
            r.take(*i);
        }
        assert_eq!(r.ranges_taken_as_vec().as_slice(),
                   [[-10_i32, -7], [8, 11]]);

        for i in &[-9, 9] {
            r.release(*i);
        }
        assert_eq!(r.ranges_taken_as_vec().as_slice(),
                   [[-10, -10], [-8, -7], [8, 8], [10, 11]]);

        for i in &[8, 10, 11] {
            r.release(*i);
        }
        assert_eq!(r.ranges_taken_as_vec().as_slice(),
                   [[-10, -10], [-8, -7]]);

        for i in &[-10, -8, -7] {
            r.release(*i);
        }
        assert!(r.is_empty());

        // r.print();
    }
}
