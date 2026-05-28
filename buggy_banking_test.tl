module FinancialEngine

type Wallet = struct {
    owner_id: String,
    balance: Int,
}

intent StrictTransfer(source: Wallet, target: Wallet, amount: Int) {
    preconditions: [
        amount > 0,
        source.balance >= amount
    ],
    postconditions: [
        source.balance == initial(source.balance) - amount,
        target.balance == initial(target.balance) + amount
    ]
}

// BUG: Mutating source balance twice or doing incorrect math
pub fn execute_payout(mut source: Wallet, mut target: Wallet, amount: Int) satisfies StrictTransfer {
    source.balance -= amount;
    target.balance += amount - 1; // Logical bug here!
}
