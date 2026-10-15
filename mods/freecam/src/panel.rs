//! A window over the game: flight options and the map list.

use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::mpsc::Sender;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{CreateFontW, CreateSolidBrush, DeleteObject, HFONT};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BS_PUSHBUTTON, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, ES_AUTOHSCROLL, GWLP_USERDATA,
    GetDlgItem, GetMessageW, GetWindowLongPtrW, GetWindowTextW, HWND_TOPMOST, LBS_NOTIFY,
    LBS_SORT, LB_ADDSTRING, LB_GETCURSEL, LB_GETTEXT, PostQuitMessage, RegisterClassW, SWP_NOSIZE,
    SW_HIDE, SW_SHOW, SendMessageW, SetWindowLongPtrW, SetWindowPos, ShowWindow, WM_COMMAND,
    WM_DESTROY, WNDCLASSW, WS_BORDER, WS_CAPTION, WS_CHILD, WS_EX_TOPMOST, WS_POPUP, WS_SYSMENU,
    WS_VISIBLE, WS_VSCROLL,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{DispatchMessageW, TranslateMessage};

use crate::maps::Map;

const TOGGLE: usize = 1;
const LOAD: usize = 2;
const LIST: usize = 3;
const SPEED: usize = 4;
const FOV: usize = 5;
const EN_CHANGE: usize = 0x0300;

pub enum Request {
    Toggle,
    Load(String),
    Speed(f32),
    Fov(f32),
}

static WINDOW: AtomicIsize = AtomicIsize::new(0);

pub fn open(requests: Sender<Request>, maps: Vec<Map>) {
    std::thread::spawn(move || run(requests, maps));
}

/// Shows the panel when `shown`, hides it otherwise.
pub fn show(shown: bool) {
    let handle = WINDOW.load(Ordering::Relaxed);
    if handle != 0 {
        unsafe { ShowWindow(handle as HWND, if shown { SW_SHOW } else { SW_HIDE }) };
    }
}

struct State {
    requests: Sender<Request>,
    maps: Vec<Map>,
}

fn run(requests: Sender<Request>, maps: Vec<Map>) {
    let class = wide("ExarustFreeCam");
    let title = wide("Exarust");
    let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
    let brush = unsafe { CreateSolidBrush(0x00181010) };
    unsafe {
        let info = WNDCLASSW {
            lpfnWndProc: Some(procedure),
            hInstance: instance,
            lpszClassName: class.as_ptr(),
            hbrBackground: brush,
            ..std::mem::zeroed()
        };
        RegisterClassW(&info);
    }
    let window = unsafe {
        CreateWindowExW(
            WS_EX_TOPMOST,
            class.as_ptr(),
            title.as_ptr(),
            WS_POPUP | WS_CAPTION | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            280,
            440,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        )
    };
    if window.is_null() {
        return;
    }
    let state = Box::leak(Box::new(State { requests, maps }));
    unsafe {
        SetWindowLongPtrW(window, GWLP_USERDATA, state as *mut State as isize);
        build(window, instance, state);
        ShowWindow(window, SW_HIDE);
        SetWindowPos(window, HWND_TOPMOST, 24, 24, 0, 0, SWP_NOSIZE);
    }
    WINDOW.store(window as isize, Ordering::Relaxed);
    loop {
        let mut message = unsafe { std::mem::zeroed() };
        if unsafe { GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) } <= 0 {
            break;
        }
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    unsafe { DeleteObject(brush) };
}

unsafe fn build(window: HWND, instance: windows_sys::Win32::Foundation::HINSTANCE, state: &State) {
    let font = unsafe { CreateFontW(16, 0, 0, 0, 400, 0, 0, 0, 0, 0, 0, 0, 0, wide("Segoe UI").as_ptr()) };
    control(window, instance, font, "BUTTON", "Free cam", BS_PUSHBUTTON as u32, 16, 12, 240, 28, TOGGLE);
    control(window, instance, font, "STATIC", "Speed cm/s", 0, 16, 48, 110, 16, 0);
    control(window, instance, font, "EDIT", "1500", ES_AUTOHSCROLL as u32 | WS_BORDER, 16, 66, 100, 22, SPEED);
    control(window, instance, font, "STATIC", "Field of view", 0, 140, 48, 110, 16, 0);
    control(window, instance, font, "EDIT", "60", ES_AUTOHSCROLL as u32 | WS_BORDER, 140, 66, 100, 22, FOV);
    control(
        window, instance, font, "LISTBOX", "",
        LBS_NOTIFY as u32 | LBS_SORT as u32 | WS_BORDER | WS_VSCROLL,
        16, 100, 240, 250, LIST,
    );
    control(window, instance, font, "BUTTON", "Load map", BS_PUSHBUTTON as u32, 16, 360, 240, 28, LOAD);
    let list = unsafe { GetDlgItem(window, LIST as i32) };
    for map in &state.maps {
        let name = wide(&map.name);
        unsafe { SendMessageW(list, LB_ADDSTRING, 0, name.as_ptr() as LPARAM) };
    }
}

fn control(
    parent: HWND,
    instance: windows_sys::Win32::Foundation::HINSTANCE,
    font: HFONT,
    class: &str,
    text: &str,
    style: u32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    id: usize,
) {
    let class = wide(class);
    let text = wide(text);
    let window = unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            text.as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            x,
            y,
            width,
            height,
            parent,
            id as _,
            instance,
            std::ptr::null(),
        )
    };
    unsafe { SendMessageW(window, 0x0030, font as WPARAM, 1) };
}

unsafe extern "system" fn procedure(window: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if message == WM_COMMAND {
        command(window, w);
    } else if message == WM_DESTROY {
        unsafe { PostQuitMessage(0) };
    }
    unsafe { DefWindowProcW(window, message, w, l) }
}

fn command(window: HWND, w: WPARAM) {
    let id = (w & 0xFFFF) as usize;
    let code = (w >> 16) & 0xFFFF;
    let Some(state) = state(window) else { return };
    match (id, code) {
        (TOGGLE, 0) => drop(state.requests.send(Request::Toggle)),
        (LOAD, 0) => {
            if let Some(name) = selected(window) {
                drop(state.requests.send(Request::Load(name)));
            }
        }
        (SPEED | FOV, EN_CHANGE) => {
            if let Some(value) = number(window, id) {
                let request = if id == SPEED { Request::Speed(value) } else { Request::Fov(value) };
                drop(state.requests.send(request));
            }
        }
        _ => {}
    }
}

fn selected(window: HWND) -> Option<String> {
    let list = unsafe { GetDlgItem(window, LIST as i32) };
    if list.is_null() {
        return None;
    }
    let index = unsafe { SendMessageW(list, LB_GETCURSEL, 0, 0) };
    if index < 0 {
        return None;
    }
    let mut buffer = [0u16; 64];
    unsafe { SendMessageW(list, LB_GETTEXT, index as WPARAM, buffer.as_mut_ptr() as LPARAM) };
    let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..end]))
}

fn number(window: HWND, id: usize) -> Option<f32> {
    let field = unsafe { GetDlgItem(window, id as i32) };
    if field.is_null() {
        return None;
    }
    let mut buffer = [0u16; 32];
    let length = unsafe { GetWindowTextW(field, buffer.as_mut_ptr(), buffer.len() as i32) };
    String::from_utf16_lossy(&buffer[..length as usize]).trim().parse().ok()
}

fn state(window: HWND) -> Option<&'static State> {
    let pointer = unsafe { GetWindowLongPtrW(window, GWLP_USERDATA) } as *const State;
    if pointer.is_null() { None } else { Some(unsafe { &*pointer }) }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
