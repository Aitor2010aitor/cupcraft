/* cupcraft — launcher de escritorio 100% C++ nativo (Win32 + WebView2 COM).
   Sustituye al antiguo launcher C#/.NET: mismo comportamiento (instancia única,
   servidor local propio, WebView2 oscuro) sin dependencia de .NET ni de DLLs
   gestionadas. Compilar con MSVC x64 y WebView2LoaderStatic.lib. */

#ifndef UNICODE
#define UNICODE
#endif
#ifndef _UNICODE
#define _UNICODE
#endif

#include <windows.h>
#include <windowsx.h>
#include <dwmapi.h>
#include <winhttp.h>
#include <shellapi.h>
#include <shlobj.h>
#include <objbase.h>

#include <string>
#include <vector>
#include <fstream>
#include <algorithm>
#include <atomic>

#include <WebView2.h>
#include <wrl.h>

using Microsoft::WRL::Callback;
using Microsoft::WRL::ComPtr;

#pragma comment(lib, "user32.lib")
#pragma comment(lib, "gdi32.lib")
#pragma comment(lib, "dwmapi.lib")
#pragma comment(lib, "winhttp.lib")
#pragma comment(lib, "ole32.lib")
#pragma comment(lib, "shell32.lib")
#pragma comment(lib, "advapi32.lib")

namespace {

constexpr wchar_t kAppUrl[]      = L"http://127.0.0.1:8787/";
constexpr wchar_t kWindowClass[] = L"openCUPDesktopWindow";
constexpr wchar_t kInstanceMutex[] = L"openCUP.Desktop.SingleInstance";
constexpr wchar_t kWindowTitle[] = L"cupcraft";

constexpr int kClientWidth  = 1460;
constexpr int kClientHeight = 900;
constexpr int kMinWidth     = 1100;
constexpr int kMinHeight    = 700;

// Colores del tema oscuro (equivalente al launcher anterior).
constexpr COLORREF kBgColor   = RGB(13, 17, 19);
constexpr COLORREF kTextColor = RGB(140, 235, 225);

constexpr int kDwmUseImmersiveDarkMode    = 20;  // Windows 10 20H1+
constexpr int kDwmUseImmersiveDarkModeOld = 19;  // builds anteriores
constexpr int kDwmWindowCornerPreference  = 33;
constexpr int kDwmWcpRound                = 2;

HINSTANCE g_instance   = nullptr;
HWND     g_mainWindow  = nullptr;
HWND     g_statusLabel = nullptr;
HANDLE   g_serverJob   = nullptr;   // proceso node (si lo arrancamos nosotros)
bool     g_ownsServer  = false;
HBRUSH   g_bgBrush     = nullptr;

std::atomic<bool> g_stoppingServer{false};   // cierre ordenado del servidor
std::vector<HANDLE> g_pendingReads;          // descriptores sin hilo lector
HANDLE   g_jobObject = nullptr;              // Job: node muere con cupcraft

ComPtr<ICoreWebView2Controller> g_controller;
ComPtr<ICoreWebView2>           g_webView;

/* ------------------------------- Registro -------------------------------- */

std::wstring DataDirectory() {
  wchar_t localAppData[MAX_PATH] = {};
  if (FAILED(SHGetFolderPathW(nullptr, CSIDL_LOCAL_APPDATA, nullptr, SHGFP_TYPE_CURRENT, localAppData))) {
    wcscpy_s(localAppData, L".");
  }
  std::wstring dir = std::wstring(localAppData) + L"\\openCUP";
  CreateDirectoryW(dir.c_str(), nullptr);
  return dir;
}

std::wstring LogPath() {
  return DataDirectory() + L"\\openCUP.log";
}

std::wstring AppDirectory() {
  wchar_t buffer[MAX_PATH] = {};
  GetModuleFileNameW(nullptr, buffer, MAX_PATH);
  std::wstring path(buffer);
  const size_t slash = path.find_last_of(L'\\');
  return slash == std::wstring::npos ? L"." : path.substr(0, slash);
}

void LogLine(const std::string& line) {
  if (line.empty()) return;
  std::wstring path = LogPath();
  std::ofstream file(path, std::ios::app);
  if (!file) return;
  SYSTEMTIME now{};
  GetLocalTime(&now);
  char stamp[32];
  snprintf(stamp, sizeof(stamp), "%04d-%02d-%02d %02d:%02d:%02d ",
           now.wYear, now.wMonth, now.wDay, now.wHour, now.wMinute, now.wSecond);
  file << stamp << line << '\n';
}

std::string WideToUtf8(const wchar_t* text) {
  if (!text) return {};
  int size = WideCharToMultiByte(CP_UTF8, 0, text, -1, nullptr, 0, nullptr, nullptr);
  if (size <= 1) return {};
  std::string utf8(size - 1, '\0');
  WideCharToMultiByte(CP_UTF8, 0, text, -1, utf8.data(), size, nullptr, nullptr);
  return utf8;
}

void LogWide(const wchar_t* prefix, const std::wstring& text) {
  LogLine(WideToUtf8(prefix) + WideToUtf8(text.c_str()));
}

/* --------------------------- Espera al servidor -------------------------- */

bool ProbeServer() {
  bool ready = false;
  HINTERNET session = WinHttpOpen(L"openCUP/1.0", WINHTTP_ACCESS_TYPE_NO_PROXY,
                                  WINHTTP_NO_PROXY_NAME, WINHTTP_NO_PROXY_BYPASS, 0);
  if (session) {
    WinHttpSetTimeouts(session, 800, 800, 800, 800);
    HINTERNET connection = WinHttpConnect(session, L"127.0.0.1", 8787, 0);
    if (connection) {
      HINTERNET request = WinHttpOpenRequest(connection, L"GET", L"/", nullptr,
                                             WINHTTP_NO_REFERER, WINHTTP_DEFAULT_ACCEPT_TYPES, 0);
      if (request) {
        if (WinHttpSendRequest(request, WINHTTP_NO_ADDITIONAL_HEADERS, 0,
                               WINHTTP_NO_REQUEST_DATA, 0, 0, 0) &&
            WinHttpReceiveResponse(request, nullptr)) {
          DWORD status = 0;
          DWORD size = sizeof(status);
          WinHttpQueryHeaders(request,
                              WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                              WINHTTP_HEADER_NAME_BY_INDEX, &status, &size,
                              WINHTTP_NO_HEADER_INDEX);
          ready = status == 200;
        }
        WinHttpCloseHandle(request);
      }
      WinHttpCloseHandle(connection);
    }
    WinHttpCloseHandle(session);
  }
  return ready;
}

bool WaitForServer(DWORD timeoutMs) {
  const ULONGLONG deadline = GetTickCount64() + timeoutMs;
  while (GetTickCount64() < deadline) {
    if (ProbeServer()) return true;
    Sleep(300);
  }
  return false;
}

/* ------------------------- Arranque del servidor ------------------------- */

std::wstring Quote(const std::wstring& value) {
  return L"\"" + value + L"\"";
}

// Variables de entorno del servidor. Se fijan en este proceso antes de crear
// el hijo (CreateProcess exige el bloque de entorno ordenado alfabéticamente,
// por lo que rehacerlo a mano falla con ERROR_INVALID_PARAMETER).
void ConfigureServerEnvironment() {
  SetEnvironmentVariableW(L"PORT", L"8787");
  SetEnvironmentVariableW(L"openCUP_DESKTOP", L"1");
}

// Volcado de stdout/stderr del servidor al registro. El hilo es dueño del
// descriptor de lectura: lo cierra al finalizar. Cerrarlo en el proceso padre
// rompería la tubería y node moriría con EPIPE al escribir.
void PumpServerOutput(HANDLE pipe) {
  char buffer[512];
  std::string pending;
  DWORD read = 0;
  while (ReadFile(pipe, buffer, sizeof(buffer), &read, nullptr) && read > 0) {
    pending.append(buffer, read);
    size_t newline;
    while ((newline = pending.find('\n')) != std::string::npos) {
      std::string line = pending.substr(0, newline);
      if (!line.empty() && line.back() == '\r') line.pop_back();
      LogLine(line);
      pending.erase(0, newline + 1);
    }
  }
  if (!pending.empty()) LogLine(pending);
  CloseHandle(pipe);
}

bool FileExists(const std::wstring& path) {
  const DWORD attrs = GetFileAttributesW(path.c_str());
  return attrs != INVALID_FILE_ATTRIBUTES && !(attrs & FILE_ATTRIBUTE_DIRECTORY);
}

void StartServer() {
  const std::wstring dir = AppDirectory();
  std::wstring node = dir + L"\\node.exe";
  if (!FileExists(node)) node = L"node.exe";  // PATH del sistema
  const std::wstring script = dir + L"\\server.mjs";
  if (!FileExists(script)) {
    LogLine("No se encontró server.mjs en " + WideToUtf8(dir.c_str()));
    return;
  }

  SECURITY_ATTRIBUTES security{ sizeof(SECURITY_ATTRIBUTES), nullptr, TRUE };
  HANDLE outRead = nullptr, outWrite = nullptr;
  HANDLE errRead = nullptr, errWrite = nullptr;
  if (!CreatePipe(&outRead, &outWrite, &security, 0) ||
      !CreatePipe(&errRead, &errWrite, &security, 0)) {
    LogLine("Servidor: no se pudieron crear las tuberías de salida");
    return;
  }
  SetHandleInformation(outRead, HANDLE_FLAG_INHERIT, 0);
  SetHandleInformation(errRead, HANDLE_FLAG_INHERIT, 0);

  std::wstring commandLine = Quote(node) + L" " + Quote(script);
  std::vector<wchar_t> command(commandLine.begin(), commandLine.end());
  command.push_back(L'\0');

  ConfigureServerEnvironment();

  STARTUPINFOW startup{};
  startup.cb = sizeof(startup);
  startup.dwFlags = STARTF_USESTDHANDLES;
  startup.hStdInput  = GetStdHandle(STD_INPUT_HANDLE);
  startup.hStdOutput = outWrite;
  startup.hStdError  = errWrite;

  PROCESS_INFORMATION process{};
  const BOOL ok = CreateProcessW(nullptr, command.data(), nullptr, nullptr, TRUE,
                                 CREATE_NO_WINDOW, nullptr,
                                 dir.c_str(), &startup, &process);
  CloseHandle(outWrite);
  CloseHandle(errWrite);
  if (!ok) {
    CloseHandle(outRead);
    CloseHandle(errRead);
    LogLine("Servidor: CreateProcess falló (error " + std::to_string(GetLastError()) + ")");
    return;
  }

  CloseHandle(process.hThread);
  g_serverJob = process.hProcess;
  g_ownsServer = true;
  // El hijo pasa al Job: si cupcraft muere, Windows lo termina con él.
  if (g_jobObject && !AssignProcessToJobObject(g_jobObject, process.hProcess)) {
    LogLine("Aviso: no se pudo asociar el servidor al Job (error " +
            std::to_string(GetLastError()) + ")");
  }
  LogLine("Servidor iniciado (PID " + std::to_string(process.dwProcessId) + ")");

  // Hilos lectores de stdout/stderr que vuelcan al registro. Son dueños de sus
  // descriptores (los cierra el propio hilo), nunca el proceso padre.
  HANDLE threadOut = CreateThread(nullptr, 0, [](LPVOID param) -> DWORD {
    PumpServerOutput(static_cast<HANDLE>(param));
    return 0;
  }, outRead, 0, nullptr);
  HANDLE threadErr = CreateThread(nullptr, 0, [](LPVOID param) -> DWORD {
    PumpServerOutput(static_cast<HANDLE>(param));
    return 0;
  }, errRead, 0, nullptr);
  if (threadOut) CloseHandle(threadOut); else g_pendingReads.push_back(outRead);
  if (threadErr) CloseHandle(threadErr); else g_pendingReads.push_back(errRead);

  // Vigilante del proceso: anota en el registro si el servidor muere solo.
  HANDLE watched = nullptr;
  if (DuplicateHandle(GetCurrentProcess(), process.hProcess, GetCurrentProcess(),
                      &watched, 0, FALSE, DUPLICATE_SAME_ACCESS)) {
    HANDLE monitor = CreateThread(nullptr, 0, [](LPVOID param) -> DWORD {
      HANDLE process_ = static_cast<HANDLE>(param);
      WaitForSingleObject(process_, INFINITE);
      DWORD code = 1;
      GetExitCodeProcess(process_, &code);
      CloseHandle(process_);
      if (!g_stoppingServer) {
        LogLine("Servidor finalizado inesperadamente (código " + std::to_string(code) + ")");
      }
      return 0;
    }, watched, 0, nullptr);
    if (monitor) CloseHandle(monitor); else CloseHandle(watched);
  }
}

// Job object: si cupcraft muere (incluso a la fuerza), Windows mata al servidor
// hijo. Evita quedarse con un node huérfano ocupando los puertos 8787/8790.
void CreateServerJob() {
  g_jobObject = CreateJobObjectW(nullptr, nullptr);
  if (!g_jobObject) return;
  JOBOBJECT_EXTENDED_LIMIT_INFORMATION info{};
  info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
  if (!SetInformationJobObject(g_jobObject, JobObjectExtendedLimitInformation,
                                &info, sizeof(info))) {
    CloseHandle(g_jobObject);
    g_jobObject = nullptr;
  }
}

void CloseServerJob() {
  if (!g_jobObject) return;
  CloseHandle(g_jobObject);   // con KILL_ON_JOB_CLOSE: termina al hijo
  g_jobObject = nullptr;
}

void StopServer() {
  if (!g_ownsServer || !g_serverJob) { CloseServerJob(); return; }
  g_stoppingServer = true;
  if (WaitForSingleObject(g_serverJob, 0) == WAIT_OBJECT_0) {
    CloseHandle(g_serverJob);
    g_serverJob = nullptr;
    g_ownsServer = false;
    CloseServerJob();
    return;
  }
  TerminateProcess(g_serverJob, 0);
  WaitForSingleObject(g_serverJob, 2500);
  CloseHandle(g_serverJob);
  g_serverJob = nullptr;
  g_ownsServer = false;
  for (HANDLE read : g_pendingReads) CloseHandle(read);
  g_pendingReads.clear();
  CloseServerJob();
}

/* ------------------------------- WebView2 -------------------------------- */

void ResizeWebView() {
  if (!g_controller || !g_mainWindow) return;
  RECT bounds{};
  GetClientRect(g_mainWindow, &bounds);
  g_controller->put_Bounds(bounds);
}

void ConfigureAndNavigate(ICoreWebView2* webView) {
  ICoreWebView2Settings* settings = nullptr;
  if (SUCCEEDED(webView->get_Settings(&settings)) && settings) {
    settings->put_AreDevToolsEnabled(FALSE);
    settings->put_AreDefaultContextMenusEnabled(FALSE);
    settings->put_IsStatusBarEnabled(FALSE);
    settings->put_IsZoomControlEnabled(TRUE);

    // Teclas de aceleración del navegador: Settings3.
    ComPtr<ICoreWebView2Settings3> settings3;
    if (SUCCEEDED(settings->QueryInterface(IID_PPV_ARGS(&settings3))) && settings3) {
      settings3->put_AreBrowserAcceleratorKeysEnabled(FALSE);
    }
    // Autoselleno de contraseñas e indicadores: Settings4.
    ComPtr<ICoreWebView2Settings4> settings4;
    if (SUCCEEDED(settings->QueryInterface(IID_PPV_ARGS(&settings4))) && settings4) {
      settings4->put_IsPasswordAutosaveEnabled(FALSE);
      settings4->put_IsGeneralAutofillEnabled(FALSE);
    }
    settings->Release();
  }
  webView->Navigate(kAppUrl);
  if (g_statusLabel) ShowWindow(g_statusLabel, SW_HIDE);
}

void InitializeWebView() {
  const std::wstring userData = DataDirectory() + L"\\WebView2";
  CreateDirectoryW(userData.c_str(), nullptr);

  const HRESULT hr = CreateCoreWebView2EnvironmentWithOptions(
      nullptr, userData.c_str(), nullptr,
      Callback<ICoreWebView2CreateCoreWebView2EnvironmentCompletedHandler>(
        [](HRESULT result, ICoreWebView2Environment* environment) -> HRESULT {
          if (FAILED(result) || !environment) {
            LogWide(L"WebView2 entorno: ", std::to_wstring(result));
            if (g_statusLabel) {
              SetWindowTextW(g_statusLabel,
                             L"No se pudo abrir la interfaz de cupcraft.\r\n"
                             L"Comprueba que WebView2 Runtime esté instalado.");
            }
            return S_OK;
          }
          environment->CreateCoreWebView2Controller(
              g_mainWindow,
              Callback<ICoreWebView2CreateCoreWebView2ControllerCompletedHandler>(
                [](HRESULT result, ICoreWebView2Controller* controller) -> HRESULT {
                  if (FAILED(result) || !controller) {
                    LogWide(L"WebView2 controlador: ", std::to_wstring(result));
                    if (g_statusLabel) {
                      SetWindowTextW(g_statusLabel,
                                     L"No se pudo abrir la interfaz de cupcraft.\r\n"
                                     L"Comprueba que WebView2 Runtime esté instalado.");
                    }
                    return S_OK;
                  }
                  g_controller = controller;
                  controller->get_CoreWebView2(&g_webView);

                  // Color de fondo oscuro: Controller2.
                  ComPtr<ICoreWebView2Controller2> controller2;
                  if (SUCCEEDED(controller->QueryInterface(IID_PPV_ARGS(&controller2))) && controller2) {
                    COREWEBVIEW2_COLOR background{ 255, 13, 17, 19 };  // A,R,G,B
                    controller2->put_DefaultBackgroundColor(background);
                  }
                  ResizeWebView();

                  if (g_webView) ConfigureAndNavigate(g_webView.Get());
                  return S_OK;
                }).Get());
          return S_OK;
        }).Get());

  if (FAILED(hr)) LogWide(L"WebView2 inicio: ", std::to_wstring(hr));
}

/* ------------------------------ Ventana ---------------------------------- */

void ApplyDarkTitleBar(HWND window) {
  int dark = 1;
  if (FAILED(DwmSetWindowAttribute(window, kDwmUseImmersiveDarkMode, &dark, sizeof(dark)))) {
    DwmSetWindowAttribute(window, kDwmUseImmersiveDarkModeOld, &dark, sizeof(dark));
  }
  const int rounded = kDwmWcpRound;
  DwmSetWindowAttribute(window, kDwmWindowCornerPreference, &rounded, sizeof(rounded));
}

LRESULT CALLBACK WindowProc(HWND window, UINT message, WPARAM wParam, LPARAM lParam) {
  switch (message) {
    case WM_CREATE:
      g_statusLabel = CreateWindowExW(
          0, L"STATIC", L"Iniciando cupcraft…",
          WS_CHILD | WS_VISIBLE | SS_CENTER | SS_CENTERIMAGE,
          0, 0, 0, 0, window, nullptr, g_instance, nullptr);
      return 0;

    case WM_SIZE: {
      if (g_statusLabel && g_mainWindow) {
        RECT rc{};
        GetClientRect(window, &rc);
        MoveWindow(g_statusLabel, 0, 0, rc.right, rc.bottom, TRUE);
      }
      ResizeWebView();
      return 0;
    }

    case WM_GETMINMAXINFO: {
      MINMAXINFO* info = reinterpret_cast<MINMAXINFO*>(lParam);
      RECT frame{};
      AdjustWindowRectEx(&frame, WS_OVERLAPPEDWINDOW, FALSE, 0);
      info->ptMinTrackSize.x = kMinWidth + (frame.right - frame.left);
      info->ptMinTrackSize.y = kMinHeight + (frame.bottom - frame.top);
      return 0;
    }

    case WM_CTLCOLORSTATIC: {
      HDC dc = reinterpret_cast<HDC>(wParam);
      SetTextColor(dc, kTextColor);
      SetBkColor(dc, kBgColor);
      return reinterpret_cast<LRESULT>(g_bgBrush);
    }

    case WM_ERASEBKGND: {
      RECT rc{};
      GetClientRect(window, &rc);
      FillRect(reinterpret_cast<HDC>(wParam), &rc, g_bgBrush);
      return 1;
    }

    case WM_DESTROY:
      if (g_controller) {
        g_controller->Close();
        g_controller.Reset();
      }
      if (g_webView) g_webView.Reset();
      StopServer();
      PostQuitMessage(0);
      return 0;

    default:
      return DefWindowProcW(window, message, wParam, lParam);
  }
}

/* -------------------------------- Main ----------------------------------- */

bool AlreadyRunning() {
  HANDLE mutex = CreateMutexW(nullptr, TRUE, kInstanceMutex);
  if (!mutex) return false;
  if (GetLastError() == ERROR_ALREADY_EXISTS) {
    CloseHandle(mutex);
    MessageBoxW(nullptr, L"cupcraft ya está abierto.", kWindowTitle,
                MB_OK | MB_ICONINFORMATION);
    return true;
  }
  return false;  // el manifiesto sigue vivo mientras vive el proceso
}

}  // namespace

int WINAPI wWinMain(HINSTANCE instance, HINSTANCE, PWSTR, int showCommand) {
  if (AlreadyRunning()) return 0;

  SetProcessDPIAware();
  CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
  g_instance = instance;
  g_bgBrush = CreateSolidBrush(kBgColor);

  // Arranca el servidor local antes de crear la ventana: si tarda, la etiqueta
  // de estado sigue pintada mientras se espera en background.
  if (!WaitForServer(1200)) { CreateServerJob(); StartServer(); }

  WNDCLASSEXW windowClass{};
  windowClass.cbSize = sizeof(windowClass);
  windowClass.style = CS_HREDRAW | CS_VREDRAW;
  windowClass.lpfnWndProc = WindowProc;
  windowClass.hInstance = instance;
  windowClass.hIcon = LoadIconW(instance, MAKEINTRESOURCEW(1));
  windowClass.hCursor = LoadCursorW(nullptr, IDC_ARROW);
  windowClass.hbrBackground = g_bgBrush;
  windowClass.lpszClassName = kWindowClass;
  RegisterClassExW(&windowClass);

  RECT frame{ 0, 0, kClientWidth, kClientHeight };
  AdjustWindowRect(&frame, WS_OVERLAPPEDWINDOW, FALSE);
  const int width = frame.right - frame.left;
  const int height = frame.bottom - frame.top;
  const int x = (GetSystemMetrics(SM_CXSCREEN) - width) / 2;
  const int y = (GetSystemMetrics(SM_CYSCREEN) - height) / 2;

  g_mainWindow = CreateWindowExW(
      0, kWindowClass, kWindowTitle, WS_OVERLAPPEDWINDOW,
      x, y, width, height, nullptr, nullptr, instance, nullptr);
  if (!g_mainWindow) {
    CoUninitialize();
    return 1;
  }

  ApplyDarkTitleBar(g_mainWindow);
  ShowWindow(g_mainWindow, showCommand);
  UpdateWindow(g_mainWindow);

  // WebView2 en segundo plano mientras se confirma el servidor.
  InitializeWebView();
  if (!g_ownsServer && !ProbeServer()) {
    const bool ready = WaitForServer(30000);
    if (!ready) {
      LogLine("El servidor local no respondió en 30 s");
      if (g_statusLabel) {
        SetWindowTextW(g_statusLabel,
                       L"No se pudo iniciar el servidor local de cupcraft.\r\n"
                       L"Revisa %LOCALAPPDATA%\\openCUP\\openCUP.log");
      }
    }
  }

  MSG message{};
  while (GetMessageW(&message, nullptr, 0, 0) > 0) {
    TranslateMessage(&message);
    DispatchMessageW(&message);
  }

  if (g_bgBrush) DeleteObject(g_bgBrush);
  CoUninitialize();
  return static_cast<int>(message.wParam);
}
