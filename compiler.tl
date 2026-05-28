module TelosStage1

type Token = struct {
    token_type: Int,
    value: Int,
}

type ParserState = struct {
    current_token_idx: Int,
    error_occurred: Bool,
}

intent ParseVerification(state: ParserState, tok: Token) {
    preconditions: [
        state.current_token_idx >= 0,
        tok.token_type > 0,
        state.error_occurred == false
    ],
    postconditions: [
        state.current_token_idx == initial(state.current_token_idx) + 1,
        state.error_occurred == false
    ]
}

pub fn consume_token(mut state: ParserState, tok: Token) satisfies ParseVerification {
    state.current_token_idx += 1;
}
