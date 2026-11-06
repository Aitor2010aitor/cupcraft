using System;
using System.Drawing;
using System.Threading.Tasks;
using System.Windows.Forms;
using Microsoft.Extensions.Hosting;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.WinForms;
using OpenCup.Server;

namespace OpenCup.Desktop;

/// <summary>
/// App de escritorio de cupcraft escrita en C#/.NET: arranca el servidor Kestrel
/// en segundo plano y muestra la interfaz en una ventana WebView2.
/// Sustituye al lanzador C++ (launcher/main.cpp) y al arranque con Node.
/// </summary>
internal static class Program
{
    private const int FirstPort = 8787;
    private const int LastPort = 8800;

    [STAThread]
    private static void Main(string[] args)
    {
        ApplicationConfiguration.Initialize();

        var root = AppContext.BaseDirectory;
        var started = ServerApp.Start(args, root, FirstPort, LastPort);
        if (started is null)
        {
            MessageBox.Show(
                $"No se pudo abrir ningún puerto entre {FirstPort} y {LastPort}.",
                "cupcraft", MessageBoxButtons.OK, MessageBoxIcon.Error);
            return;
        }

        var (app, port) = started.Value;
        if (args.Contains("--server-only"))
        {
            app.WaitForShutdown();
            return;
        }

        using var form = new MainForm($"http://127.0.0.1:{port}/");
        Application.Run(form);

        app.StopAsync().GetAwaiter().GetResult();
    }
}

internal sealed class MainForm : Form
{
    private readonly WebView2 _web = new() { Dock = DockStyle.Fill };
    private readonly string _url;

    public MainForm(string url)
    {
        _url = url;
        Text = "cupcraft";
        StartPosition = FormStartPosition.CenterScreen;
        ClientSize = new Size(1440, 900);
        MinimumSize = new Size(960, 600);
        BackColor = Color.FromArgb(18, 18, 18);
        Controls.Add(_web);
        Load += async (_, _) => await InitAsync();
    }

    private async Task InitAsync()
    {
        try
        {
            var env = await CoreWebView2Environment.CreateAsync(null, null);
            await _web.EnsureCoreWebView2Async(env);
            _web.CoreWebView2.Settings.AreDefaultContextMenusEnabled = false;
            _web.CoreWebView2.Settings.IsStatusBarEnabled = false;
            _web.CoreWebView2.NewWindowRequested += (_, e) =>
            {
                e.Handled = true;
                if (Uri.TryCreate(e.Uri, UriKind.Absolute, out var uri) &&
                    (uri.Scheme == Uri.UriSchemeHttps || uri.Scheme == Uri.UriSchemeHttp))
                {
                    try { System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo(uri.AbsoluteUri) { UseShellExecute = true }); }
                    catch { /* sin navegador disponible */ }
                }
            };
            _web.Source = new Uri(_url);
        }
        catch (Exception ex)
        {
            MessageBox.Show(
                $"No se pudo iniciar WebView2.\r\n\r\n{ex.Message}\r\n\r\nInstala el runtime de Microsoft Edge WebView2.",
                "cupcraft", MessageBoxButtons.OK, MessageBoxIcon.Error);
            Close();
        }
    }
}
