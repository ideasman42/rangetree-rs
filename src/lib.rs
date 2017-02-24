// Apache License, Version 2.0
// (c) Campbell Barton, 2016

/// `RangeTree` (1d) for integer values.
///

mod mempool_elem;

use std::ptr;

// disable for slow, full-list look-ups.
const USE_BTREE: bool = true;

// ----------------------------------------------------------------------------
// Mini API, avoids using `num` crate.
//
// Exposes:
// - zero()
// - one()

/// Zero value (predefined as 0 for integer types).
pub trait Zero: Sized {
    fn zero() -> Self;
}

/// Unit value (predefined as 1 for integer types).
pub trait One: Sized {
    fn one() -> Self;
}

macro_rules! zero_one_impl {
    ($($t:ty)*) => ($(
        impl Zero for $t {
            #[inline]
            fn zero() -> Self { 0 }
        }
        impl One for $t {
            #[inline]
            fn one() -> Self { 1 }
        }
    )*)
}
zero_one_impl! { u8 u16 u32 u64 usize i8 i16 i32 i64 isize }


// ----------------------------------------------------------------------------
// Generic Range Type Traits

/// Range Type: orderable type used for items in the range tree.
// workaround so these modules can be private,
// and also used by modules here
mod types {
    use super::{
        One,
        Zero,
    };
    use mempool_elem;
    use std::ptr;
    use std::ops;

    pub trait RType:
        Ord +
        Zero +
        One +
        Copy +
        ops::Add<Output=Self> +
        ops::Sub<Output=Self> +
        ops::AddAssign +
        ops::SubAssign +
        ::std::fmt::Display +
        {}
    impl<TOrd> RType for TOrd where TOrd:
        Ord +
        Zero +
        One +
        Copy +
        ops::Add<Output=TOrd> +
        ops::Sub<Output=TOrd> +
        ops::AddAssign +
        ops::SubAssign +
        ::std::fmt::Display +
        {}

    pub struct Node<TOrd: RType> {
        // next is also used for RangeTree.free chain.
        // when blocks are unused.
        pub next: *mut Node<TOrd>,
        pub prev: *mut Node<TOrd>,

        // range: min,max (inclusive)
        pub range: [TOrd; 2],

        // rbtree
        pub left: *mut Node<TOrd>,
        pub right: *mut Node<TOrd>,
        pub color: bool,
    }

    impl<TOrd: RType> mempool_elem::MemElemUtils for Node<TOrd> {
        fn free_ptr_get(
            &self,
        ) -> *mut Node<TOrd> {
            self.next
        }
        fn free_ptr_set(
            &mut self,
            ptr: *mut Node<TOrd>,
        ) {
            self.next = ptr;
            self.prev = self;  // tag as free'd
        }
    }

    impl<TOrd: RType> Default for Node<TOrd> {
        fn default() -> Node<TOrd> {
            Node {
                next: ptr::null_mut(),
                prev: ptr::null_mut(),
                range: [TOrd::zero(), TOrd::zero()],
                left: ptr::null_mut(),
                right: ptr::null_mut(),
                // always overwritten when added to the tree
                color: false,
            }
        }
    }

    pub struct List<TOrd: RType> {
        pub first: *mut Node<TOrd>,
        pub last: *mut Node<TOrd>,
    }
}

use types::{
    Node,
    List,
    RType,
};

/// Main range-tree structure.
pub struct RangeTree<TOrd: RType> {
    range: [TOrd; 2],
    list: List<TOrd>,

    // btree root (USE_BTREE)
    root: *mut Node<TOrd>,

    node_pool: mempool_elem::MemPool<Node<TOrd>>,
}


// ----------------------------------------------------------------------------
// List API

impl<TOrd: RType> List<TOrd> {

    fn push_front(
        &mut self,
        node: *mut Node<TOrd>,
    ) {
        let node = unsafe { &mut *node };
        debug_assert!(node.next.is_null() &&
                      node.prev.is_null());
        if !self.first.is_null() {
            unsafe {
                node.next = self.first;
                (*self.first).prev = node;
                node.prev = ptr::null_mut();
            }
        } else {
            self.last = node;
        }
        self.first = node;
    }

    fn push_back(
        &mut self,
        node: *mut Node<TOrd>,
    ) {
        let node = unsafe { &mut *node };
        debug_assert!(node.next.is_null() &&
                      node.prev.is_null());
        if !self.first.is_null() {
            unsafe {
                node.prev = self.last;
                (*self.last).next = node;
                node.next = ptr::null_mut();
            }
        } else {
            self.first = node;
        }
        self.last = node;
    }

    fn push_after(
        &mut self,
        node_prev: *mut Node<TOrd>,
        node_new: *mut Node<TOrd>,
    ) {
        let node_new = unsafe { &mut *node_new };
        // node_new after node_prev

        // empty list
        if self.first.is_null() {
            self.first = node_new;
            self.last = node_new;

            debug_assert!(node_new.next.is_null() &&
                          node_new.prev.is_null());
        } else if node_prev.is_null() {
            // insert at head of list
            unsafe {
                node_new.prev = ptr::null_mut();
                node_new.next = self.first;
                (*self.first).prev = node_new;
                self.first = node_new;
            }
        } else {
            // at end of list
            let node_prev = unsafe { &mut *node_prev };
            if self.last == node_prev {
                self.last = node_new;
            }

            unsafe {
                node_new.next = node_prev.next;
                node_new.prev = node_prev;
                node_prev.next = node_new;
                let node_new_next = node_new.next;
                if !node_new_next.is_null() {
                    (*node_new_next).prev = node_new;
                }
            }
        }
    }

    fn push_before(
        &mut self,
        node_next: *mut Node<TOrd>,
        node_new: *mut Node<TOrd>,
    ) {
        let node_new = unsafe { &mut *node_new };
        // node_new before node_next

        // empty list
        if self.first.is_null() {
            self.first = node_new;
            self.last = node_new;
            debug_assert!(node_new.next.is_null() &&
                          node_new.prev.is_null());
        } else if node_next.is_null() {
            // insert at end of list
            unsafe {
                node_new.prev = self.last;
                node_new.next = ptr::null_mut();
                (*self.last).next = node_new;
                self.last = node_new;
            }
        } else {
            // at beginning of list
            let node_next = unsafe { &mut *node_next };
            if self.first == node_next {
                self.first = node_new;
            }

            unsafe {
                node_new.next = node_next;
                node_new.prev = node_next.prev;
                node_next.prev = node_new;
                let node_new_prev = node_new.prev;
                if !node_new_prev.is_null() {
                    (*node_new_prev).next = node_new;
                }
            }
        }
    }

    fn remove(
        &mut self,
        node: *mut Node<TOrd>,
    ) {
        let node = unsafe { &mut *node };
        {
            let node_next = node.next;
            if !node_next.is_null() {
                unsafe {
                    (*node_next).prev = node.prev;
                }
            }
            let node_prev = node.prev;
            if !node_prev.is_null() {
                unsafe {
                    (*node_prev).next = node.next;
                }
            }

            if self.last == node {
                self.last = node.prev;
            }
            if self.first == node {
                self.first = node.next;
            }
        }
    }

    fn clear(
        &mut self,
    ) {
        self.first = ptr::null_mut();
        self.last = ptr::null_mut();
    }

}

// ----------------------------------------------------------------------------
// BTree API

mod rb {
    use std::{
        ptr,
    };

    use types::{
        Node,
        RType,
    };

    const RED: bool = false;
    const BLACK: bool = true;

    macro_rules! key {
        ($body:expr) => {
            &$body.range[0]
        }
    }

    fn is_red<TOrd: RType>(node: *mut Node<TOrd>) -> bool
    {
        !node.is_null() && unsafe { (*node).color } == RED
    }

    fn key_cmp<TOrd: RType>(
        key1: &TOrd,
        key2: &TOrd,
    ) -> i32 {
        if key1 == key2 {
            0
        } else if key1 < key2 {
            -1
        } else {
            1
        }
    }

    fn rotate_left<TOrd: RType>(
        left: *mut Node<TOrd>,
    ) -> *mut Node<TOrd> {
        let left = unsafe { &mut *left };
        let right = unsafe { &mut *left.right };
        left.right = right.left;
        right.left = left;
        right.color = left.color;
        left.color = RED;
        right
    }

    fn rotate_right<TOrd: RType>(
        right: *mut Node<TOrd>,
    ) -> *mut Node<TOrd> {
        let right = unsafe { &mut *right };
        let left = unsafe { &mut *right.left };
        right.left = left.right;
        left.right = right;
        left.color = right.color;
        right.color = RED;
        left
    }

    fn flip_color<TOrd: RType>(
        node: *mut Node<TOrd>,
    ) {
        let node = unsafe { &mut *node };
        let left = unsafe { &mut *node.left };
        let right = unsafe { &mut *node.right };
        node.color = !node.color;
        left.color = !left.color;
        right.color = !right.color;
    }

    fn move_red_to_left<TOrd: RType>(
        mut node: *mut Node<TOrd>,
    ) -> *mut Node<TOrd> {
        // Assuming that h is red and both h.left and h.left.left
        // are black, make h.left or one of its children red.
        flip_color(node);
        if unsafe { !(*node).right.is_null() && is_red((*(*node).right).left) } {
            unsafe {
                (*node).right = rotate_right((*node).right);
            }
            node = rotate_left(node);
            flip_color(node);
        }
        node
    }

    fn move_red_to_right<TOrd: RType>(
        mut node: *mut Node<TOrd>,
    ) -> *mut Node<TOrd> {
        // Assuming that h is red and both h.right and h.right.left
        // are black, make h.right or one of its children red.
        flip_color(node);
        if unsafe { !(*node).left.is_null() && is_red((*(*node).left).left) } {
            node = rotate_right(node);
            flip_color(node);
        }
        node
    }

    pub fn insert_root<TOrd: RType>(
        mut root: *mut Node<TOrd>,
        node_to_insert: *mut Node<TOrd>,
    ) -> *mut Node<TOrd> {
        unsafe fn insert_recursive<TOrd: RType>(
            mut node: *mut Node<TOrd>,
            node_to_insert: *mut Node<TOrd>,
        ) -> *mut Node<TOrd> {
            if node.is_null() {
                return node_to_insert;
            }
            // let node = unsafe { &mut *node };

            let cmp = key_cmp(key!(*node_to_insert), key!(*node));
            if cmp == -1 {
                let left = insert_recursive((*node).left, node_to_insert);
                (*node).left = left;
            } else if cmp == 1 {
                let right = insert_recursive((*node).right, node_to_insert);
                (*node).right = right;
            } else {
                // we know this key won't already exist
                unreachable!();
            }

            if is_red((*node).right) && !is_red((*node).left) {
                node = rotate_left(node);
            }
            if is_red((*node).left) && is_red((*(*node).left).left) {
                node = rotate_right(node);
            }

            if is_red((*node).left) && is_red((*node).right) {
                flip_color(node);
            }

            node
        }

        unsafe {
            (*node_to_insert).color = RED;
            root = insert_recursive(root, node_to_insert);
            (*root).color = BLACK;
        }
        root
    }

    fn fixup_remove<TOrd: RType>(
        mut node: *mut Node<TOrd>,
    ) -> *mut Node<TOrd> {
        unsafe {
            if is_red((*node).right) {
                node = rotate_left(node);
            }
            if is_red((*node).left) && is_red((*(*node).left).left) {
                node = rotate_right(node);
            }
            if is_red((*node).left) && is_red((*node).right) {
                flip_color(node);
            }
            node
        }
    }

    fn pop_min_recursive<TOrd: RType>(
        mut node: *mut Node<TOrd>,
    ) -> (*mut Node<TOrd>, *mut Node<TOrd>) {
        unsafe {
            if node.is_null() {
                return (ptr::null_mut(), ptr::null_mut());
            } else if (*node).left.is_null() {
                return (ptr::null_mut(), node);
            } else if (!is_red((*node).left)) &&
                      (!is_red((*(*node).left).left))
            {
                node = move_red_to_left(node);
            }

            let (node_left, node_free) = pop_min_recursive((*node).left);
            (*node).left = node_left;
            (fixup_remove(node), node_free)
        }
    }

    pub fn remove_root<TOrd: RType>(
        mut root: *mut Node<TOrd>,
        node_to_remove: *mut Node<TOrd>,
    ) -> *mut Node<TOrd> {

        unsafe fn remove_recursive<TOrd: RType>(
            mut node: *mut Node<TOrd>,
            node_to_remove: *mut Node<TOrd>,
        ) -> *mut Node<TOrd> {
            if node.is_null() {
                return ptr::null_mut();
            }

            if key_cmp(key!(*node_to_remove), key!(*node)) == -1 {
                if !(*node).left.is_null() && !is_red((*node).left) && !is_red((*(*node).left).left) {
                    node = move_red_to_left(node);
                }
                (*node).left = remove_recursive((*node).left, node_to_remove);
            } else {
                if is_red((*node).left) {
                    node = rotate_right(node);
                }
                if (node == node_to_remove) && ((*node).right.is_null()) {
                    // 'node' removed
                    return ptr::null_mut();
                }
                debug_assert!(!(*node).right.is_null());
                if (!is_red((*node).right)) &&
                   (!is_red((*(*node).right).left))
                {
                    node = move_red_to_right(node);
                }

                if node == node_to_remove {
                    // minor improvement over original method
                    // no need to double lookup min
                    let (
                        node_right,
                        node_free,
                    ) = pop_min_recursive((*node).right);
                    (*node).right = node_right;

                    (*node_free).left = (*node).left;
                    (*node_free).right = (*node).right;
                    (*node_free).color = (*node).color;

                    node = node_free;
                } else {
                    (*node).right = remove_recursive((*node).right, node_to_remove);
                }
                // 'node' removed
            }
            fixup_remove(node)
        }

        unsafe {
            root = remove_recursive(root, node_to_remove);
            if !root.is_null() {
                (*root).color = BLACK;
            }
        }
        root
    }

    pub fn get_or_lower<TOrd: RType>(
        root: *mut Node<TOrd>,
        key: &TOrd,
    ) -> *mut Node<TOrd> {
        unsafe fn get_or_lower_recursive<TOrd: RType>(
            n: *mut Node<TOrd>,
            key: &TOrd,
        ) -> *mut Node<TOrd> {
            // Check if (n.key >= key)
            // to get the node directly after 'key'
            // return best node and key_lower
            let cmp_lower = key_cmp(key!(*n), key);
            if cmp_lower == 0 {
                n // exact match
            } else if cmp_lower == -1 {
                debug_assert!(key!(*n) <= &key);
                // n is greater than our best so far
                if !(*n).right.is_null() {
                    let n_test = get_or_lower_recursive((*n).right, key);
                    if !n_test.is_null() {
                        return n_test;
                    }
                }
                n
            } else {  // -1
                if !(*n).left.is_null() {
                    return get_or_lower_recursive((*n).left, key);
                }
                ptr::null_mut()
            }
        }

        unsafe {
            if !root.is_null() {
                return get_or_lower_recursive(root, key);
            }
        }
        ptr::null_mut()
    }

    // External tree API
    pub fn get_or_upper<TOrd: RType>(
        root: *mut Node<TOrd>,
        key: &TOrd,
    ) -> *mut Node<TOrd> {
        unsafe fn get_or_upper_recursive<TOrd: RType>(
            n: *mut Node<TOrd>,
            key: &TOrd,
        ) -> *mut Node<TOrd> {
            // Check if (n.key >= key)
            // to get the node directly after 'key'
            // return best node and key_upper
            let cmp_upper = key_cmp(key!(*n), key);
            if cmp_upper == 0 {
                n // exact match
            } else if cmp_upper == 1 {
                debug_assert!(key!(*n) >= key);
                // n is lower than our best so far
                if !(*n).left.is_null() {
                    let n_test = get_or_upper_recursive((*n).left, key);
                    if !n_test.is_null() {
                        return n_test;
                    }
                }
                n
            } else {  // -1
                if !(*n).right.is_null() {
                    return get_or_upper_recursive((*n).right, key);
                }
                ptr::null_mut()
            }
        }

        unsafe {
            if !root.is_null() {
                return get_or_upper_recursive(root, key);
            }
        }
        ptr::null_mut()
    }

    pub fn is_balanced<TOrd: RType>(
        root: *mut Node<TOrd>,
    ) -> bool {

        fn is_balanced_recursive<TOrd: RType>(
            node: *mut Node<TOrd>,
            mut black: isize,
        ) -> bool {
            if node.is_null() {
                return black == 0;
            }
            if !is_red(node) {
                black -= 1;
            }
            is_balanced_recursive(unsafe { (*node).left }, black) &&
            is_balanced_recursive(unsafe { (*node).right }, black)
        }

        let mut black: isize = 0;
        let mut node = root;
        while !node.is_null() {
            if !is_red(node) {
                black += 1;
            }
            node = unsafe { (*node).left };
        }
        is_balanced_recursive(root, black)
    }


}


// ----------------------------------------------------------------------------
// List API


impl<TOrd: RType> RangeTree<TOrd> {

    // ----------------------------------
    // Small take/drop API to reuse nodes

    #[inline]
    fn node_alloc(
        &mut self,
        node_data: Node<TOrd>,
    ) -> *mut Node<TOrd> {
        self.node_pool.alloc_elem_from(node_data)
    }
    #[inline]
    fn node_free(
        &mut self,
        node: *mut Node<TOrd>,
    ) {
        self.node_pool.free_elem(unsafe { &mut *node });
    }

    // ------------------------------------------------------------------------
    // Tree API: USE_BTREE

    fn tree_insert(
        &mut self,
        node: *mut Node<TOrd>,
    ) {
        debug_assert!(unsafe { (*node).left.is_null() &&
                               (*node).right.is_null() });
        self.root = rb::insert_root(self.root, node);
        debug_assert!(rb::is_balanced(self.root));
    }

    fn tree_remove(
        &mut self,
        node: *mut Node<TOrd>,
    ) {
        self.root = rb::remove_root(self.root, node);
        debug_assert!(rb::is_balanced(self.root));
    }

    fn tree_clear(
        &mut self,
    ) {
        if USE_BTREE {
            self.root = ptr::null_mut();
        }
    }

    // ------------------------------------------------------------------------
    // Node API

    fn node_add_front(
        &mut self,
        range: [TOrd; 2],
    ) {
        let node = self.node_alloc(RangeTree::new_node(range));
        self.list.push_front(node);
        if USE_BTREE {
            self.tree_insert(node);
        }
    }

    fn node_add_back(
        &mut self,
        range: [TOrd; 2],
    ) {
        let node = self.node_alloc(RangeTree::new_node(range));
        self.list.push_back(node);
        if USE_BTREE {
            self.tree_insert(node);
        }
    }

    fn node_add_before(
        &mut self,
        node_next: *mut Node<TOrd>,
        range: [TOrd; 2],
    ) {
        let node = self.node_alloc(RangeTree::new_node(range));
        self.list.push_before(node_next, node);
        if USE_BTREE {
            self.tree_insert(node);
        }
    }

    fn node_add_after(
        &mut self,
        node_prev: *mut Node<TOrd>,
        range: [TOrd; 2],
    ) {
        let node = self.node_alloc(RangeTree::new_node(range));
        self.list.push_after(node_prev, node);
        if USE_BTREE {
            self.tree_insert(node);
        }
    }

    fn node_remove(
        &mut self,
        node: *mut Node<TOrd>,
    ) {
        if USE_BTREE {
            self.tree_remove(node);
        }
        self.list.remove(node);
        self.node_free(node);
    }

    fn new_empty(
        range: [TOrd; 2],
    ) -> RangeTree<TOrd> {
        RangeTree {
            range: range,
            list: List {
                first: ptr::null_mut(),
                last: ptr::null_mut(),
            },
            node_pool: mempool_elem::MemPool::new(1024),

            // USE_BTREE
            root: ptr::null_mut(),
        }
    }

    fn new_node(
        range: [TOrd; 2],
    ) -> Node<TOrd> {
        Node {
            next: ptr::null_mut(),
            prev: ptr::null_mut(),

            range: range,

            left: ptr::null_mut(),
            right: ptr::null_mut(),
            color: false,
        }
    }

    fn find_node_from_value(
        &self,
        value: &TOrd,
    ) -> *mut Node<TOrd> {
        if USE_BTREE {
            let node = rb::get_or_lower(self.root, value);
            if !node.is_null() {
                let node = unsafe { &mut *node };
                if (value >= &node.range[0]) &&
                   (value <= &node.range[1])
                {
                    return node
                }
            }
            ptr::null_mut()
        } else {
            let mut node = self.list.first;
            while !node.is_null() {
                if (value >= unsafe { &(*node).range[0] } ) &&
                   (value <= unsafe { &(*node).range[1] } )
                {
                    return node;
                }
                node = unsafe { (*node).next };
            }
            ptr::null_mut()
        }
    }

    fn find_node_pair_around_value(
        &self,
        value: &TOrd,
    ) -> (*mut Node<TOrd>, *mut Node<TOrd>) {
        if value < unsafe { &(*(self.list.first)).range[0] } {
            return (ptr::null_mut(), self.list.first);
        } else if value > unsafe { &(*(self.list.last)).range[1] } {
            return (self.list.last, ptr::null_mut());
        } else if USE_BTREE {
            let node_next = rb::get_or_upper(self.root, value);
            if !node_next.is_null() {
                let node_next = unsafe { &mut *node_next };
                let node_prev = unsafe { &mut *(*node_next).prev };
                if (&node_prev.range[1] < value) &&
                   (&node_next.range[0] > value)
                {
                    return (node_prev, node_next)
                }
            }
        } else {
            let mut node_prev = self.list.first;
            let mut node_next = unsafe { (*node_prev).next };
            while !node_next.is_null() {
                if unsafe {(&(*node_prev).range[1] < value) &&
                           (&(*node_next).range[0] > value) }
                {
                    return (node_prev, node_next)
                }
                node_prev = node_next;
                node_next = unsafe { (*node_next).next };
            }
        }
        (ptr::null_mut(), ptr::null_mut())
    }

    /// Create a new range tree.
    ///
    /// * `range` the [minimum, maximum] values (inclusive), for this range tree.
    /// * `full` When true, the tree is created with all values *taken*.
    pub fn new(
        range: [TOrd; 2],
        full: bool,
    ) -> RangeTree<TOrd> {
        let mut r = RangeTree::new_empty(range);
        if !full {
            r.node_add_front(range);
        }
        r
    }

    /// Clear an existing range tree.
    ///
    /// * `full` When true, the tree is reset with all values *taken*.
    pub fn clear(
        &mut self,
        full: bool,
    ) {
        self.list.clear();
        self.tree_clear();
        self.node_pool.clear();

        let range = [self.range[0], self.range[1]];
        if !full {
            self.node_add_front(range);
        }
    }

    fn take_impl(
        &mut self,
        value: TOrd,
        node: *mut Node<TOrd>,
    ) {
        unsafe {
            if (*node).range[0] == value {
                if (*node).range[1] != value {
                    (*node).range[0] += TOrd::one();
                } else {
                    debug_assert!((*node).range[0] == (*node).range[1]);
                    self.node_remove(node);
                }
            }
            else if (*node).range[1] == value {
                (*node).range[1] -= TOrd::one();
            } else {
                let range_next: [TOrd; 2] = [value + TOrd::one(), (*node).range[1]];
                (*node).range[1] = value - TOrd::one();
                self.node_add_after(node, range_next);
            }
        }
    }

    /// Take a value from the tree.
    ///
    /// Note: taking a value which is already taken will panic.
    /// use `retake` in cases when its not know.
    pub fn take(
        &mut self,
        value: TOrd,
    ) {
        let node = self.find_node_from_value(&value);
        debug_assert!(!node.is_null());
        self.take_impl(value, node);
    }

    /// Take a value which may already be taken,
    /// returning true if the value didn't already exist in the tree.
    pub fn retake(
        &mut self,
        value: TOrd,
    ) -> bool {
        let node = self.find_node_from_value(&value);
        if !node.is_null() {
            self.take_impl(value, node);
            true
        } else {
            false
        }
    }

    /// Take any value from the range tree.
    pub fn take_any(
        &mut self,
    ) -> Option<TOrd> {
        if !self.list.first.is_null() {
            let node = self.list.first;
            let value = unsafe { (*node).range[0] };
            if value == unsafe { (*node).range[1] } {
                self.node_remove(node);
            } else {
                unsafe {
                    (*self.list.first).range[0] += TOrd::one();
                }
            }
            Some(value)
        } else {
            None
        }
    }

    /// Check if the tree has this value (not taken).
    pub fn has(
        &self,
        value: TOrd,
    ) -> bool {
        if (value < self.range[0]) ||
           (value > self.range[1])
        {
            return true;
        }
        let node = self.find_node_from_value(&value);
        !node.is_null()
    }

    /// Check if no values in the tree are taken.
    pub fn is_empty(
        &self,
    ) -> bool {
        if self.list.first.is_null() {
            return false;  // NULL
        }
        (self.list.first == self.list.last) &&
        (unsafe { self.range[0] == (*self.list.first).range[0] }) &&
        (unsafe { self.range[1] == (*self.list.first).range[1] })
    }

    /// Check if all values in the tree are taken.
    pub fn is_full(
        &self,
    ) -> bool {
        self.list.first.is_null()
    }

    /// Release a value that has been taken.
    pub fn release(
        &mut self,
        value: TOrd,
    ) {
        let (
            touch_prev,
            touch_next,
            node_prev,
            node_next,
        ) = {
            if !self.list.first.is_null() {
                let (
                    node_prev,
                    node_next,
                ) = self.find_node_pair_around_value(&value);
                /* the value must have been already taken */
                debug_assert!(!(node_prev.is_null() && node_next.is_null()));

                /* Cases:
                 * 1) fill the gap between prev & next (two spans into one span).
                 * 2) touching prev, (grow prev.max up one).
                 * 3) touching next, (grow next.min down one).
                 * 4) touching neither, add a new segment. */
                (
                    (!node_prev.is_null() &&
                     unsafe { ((*node_prev).range[1] + TOrd::one()) == value }),
                    (!node_next.is_null() &&
                     unsafe { ((*node_next).range[0] - TOrd::one()) == value }),
                    node_prev,
                    node_next,
                )
            } else {
                // we could handle this case (4) inline,
                // since its not a common case - use regular logic.
                (false, false, ptr::null_mut(), ptr::null_mut())
            }
        };

        unsafe {
            if touch_prev && touch_next {
                // case 1:
                (*node_prev).range[1] = (*node_next).range[1];
                self.node_remove(node_next);
            } else if touch_prev {
                // case 2:
                debug_assert!(((*node_prev).range[1] + TOrd::one()) == value);
                (*node_prev).range[1] = value;
            } else if touch_next {
                // case 3:
                debug_assert!(((*node_next).range[0] - TOrd::one()) == value);
                (*node_next).range[0] = value;
            } else {
                // case 4:
                let range_new = [value, value];
                if !node_prev.is_null() {
                    self.node_add_after(node_prev, range_new);
                } else if !node_next.is_null() {
                    self.node_add_before(node_next, range_new);
                } else {
                    debug_assert!(self.list.first.is_null());
                    self.node_add_back(range_new);
                }
            }
        }
    }

    /// Return a vector containing [minimum, maximum] pairs (inclusive)
    /// of contiguous ranges which have been taken.
    pub fn ranges_taken_as_vec(
        &self,
    ) -> Vec<[TOrd; 2]> {
        let mut ret: Vec<[TOrd; 2]> = vec![];
        if self.is_empty() {
            // pass
        } else if self.list.first.is_null() {
            ret.push(self.range);
        } else {
            unsafe {
                if (*self.list.first).range[0] != self.range[0] {
                    ret.push([
                        self.range[0],
                        (*self.list.first).range[0] - TOrd::one(),
                    ]);
                }
            }

            unsafe {
                let mut node_prev = self.list.first;
                let mut node_next = (*node_prev).next;
                while !node_next.is_null() {
                    ret.push([
                        (*node_prev).range[1] + TOrd::one(),
                        (*node_next).range[0] - TOrd::one(),
                    ]);
                    node_prev = node_next;
                    node_next = (*node_next).next;
                }
            }

            unsafe {
                if (*self.list.last).range[1] != self.range[1] {
                    ret.push([
                        (*self.list.last).range[1] + TOrd::one(),
                        self.range[1],
                    ]);
                }
            }
        }

        ret
    }


    /// Return a vector containing [minimum, maximum] pairs (inclusive)
    /// of contiguous ranges which have not been taken.
    pub fn ranges_untaken_as_vec(
        &self,
    ) -> Vec<[TOrd; 2]> {
        let mut ret: Vec<[TOrd; 2]> = vec![];
        if self.is_empty() {
            ret.push(self.range);
        } else if self.list.first.is_null() {
            // pass
        } else {
            unsafe {
                let mut node = self.list.first;
                while !node.is_null() {
                    ret.push([
                        (*node).range[0],
                        (*node).range[1],
                    ]);
                    node = (*node).next;
                }
            }
        }

        ret
    }

    #[allow(dead_code)]
    fn print(
        &self,
    ) {
        let mut node = self.list.first;
        print!("print: [");
        while !node.is_null() {
            unsafe {
                print!("[{}, {}], ", (*node).range[0], (*node).range[1]);
                node = (*node).next;
            }
        }
        println!("]");
    }
}

#[cfg(test)]
mod tests_mempool;
