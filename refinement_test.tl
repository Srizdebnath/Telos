module RefinementModule

type PositiveInt = Int where value > 0

intent DoublePositive(x: PositiveInt) {
    preconditions: [],
    postconditions: [
        x > 0
    ]
}

pub fn get_double(mut x: PositiveInt) satisfies DoublePositive {
    // Correct refinement propagation
}

intent BuggyIntent(x: Int) {
    preconditions: [],
    postconditions: []
}

pub fn buggy_assignment(mut x: Int) satisfies BuggyIntent {
    let mut y: PositiveInt = -5; // Violates y > 0 refinement constraint!
}
