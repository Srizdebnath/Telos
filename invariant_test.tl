module InvariantModule

type Account = struct {
    balance: Int,
}

intent StrictInvariants(acc: Account, amount: Int) {
    preconditions: [
        acc.balance >= 0,
        amount >= 0
    ],
    postconditions: [],
    invariants: [
        acc.balance >= 0
    ]
}

pub fn deposit(mut acc: Account, amount: Int) satisfies StrictInvariants {
    acc.balance += amount;
}

pub fn buggy_withdraw(mut acc: Account, amount: Int) satisfies StrictInvariants {
    acc.balance -= amount; // Bug: may violate invariant (acc.balance >= 0) if amount > balance!
}
