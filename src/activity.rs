use winapi::um::winuser::LASTINPUTINFO;
pub fn get_last_input() -> LASTINPUTINFO {
    let mut input_info = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    unsafe { winapi::um::winuser::GetLastInputInfo(&mut input_info as *mut _) };
    input_info
}
pub async fn get_input() {
    let input = get_last_input();
    while input.dwTime == get_last_input().dwTime {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
