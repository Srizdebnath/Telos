type UserData = struct {
    id: Int,
}

pub fn process_payload(mut payload: UserData) {
    let internal_ref = &payload;
    let mut_ref = &mut payload;
}
