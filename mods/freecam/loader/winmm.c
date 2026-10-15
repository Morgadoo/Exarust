/* Sits next to Exanima.exe as winmm.dll. The game imports timeBeginPeriod and
 * timeEndPeriod from that name, so this forwards both to the system winmm.dll
 * and then loads exa_freecam.dll from the same folder.
 *
 * enable is called on a new thread, after DllMain returns. Calling it under
 * the loader lock closes the game. */

#define WIN32_LEAN_AND_MEAN
#include <windows.h>

static HMODULE real_winmm;

static FARPROC real(const char *name) {
    if (!real_winmm) {
        wchar_t path[MAX_PATH];
        GetSystemDirectoryW(path, MAX_PATH);
        lstrcatW(path, L"\\winmm.dll");
        real_winmm = LoadLibraryW(path);
    }
    return GetProcAddress(real_winmm, name);
}

__declspec(dllexport) unsigned int __stdcall timeBeginPeriod(unsigned int period) {
    typedef unsigned int (__stdcall *Fn)(unsigned int);
    Fn fn = (Fn)real("timeBeginPeriod");
    return fn ? fn(period) : 0;
}

__declspec(dllexport) unsigned int __stdcall timeEndPeriod(unsigned int period) {
    typedef unsigned int (__stdcall *Fn)(unsigned int);
    Fn fn = (Fn)real("timeEndPeriod");
    return fn ? fn(period) : 0;
}

static wchar_t mod_path[MAX_PATH];

static DWORD WINAPI start(LPVOID unused) {
    (void)unused;
    HMODULE mod = LoadLibraryW(mod_path);
    if (mod) {
        FARPROC enable = GetProcAddress(mod, "enable");
        if (enable) ((void (__stdcall *)(void))enable)();
    }
    return 0;
}

BOOL WINAPI DllMain(HINSTANCE self, DWORD reason, LPVOID reserved) {
    (void)reserved;
    if (reason == DLL_PROCESS_ATTACH) {
        GetModuleFileNameW(self, mod_path, MAX_PATH);
        wchar_t *slash = mod_path;
        for (wchar_t *p = mod_path; *p; p++) if (*p == L'\\') slash = p;
        lstrcpyW(slash + 1, L"exa_freecam.dll");
        CloseHandle(CreateThread(NULL, 0, start, NULL, 0, NULL));
    }
    return TRUE;
}
