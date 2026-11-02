using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.FileProviders;
using Microsoft.Extensions.Hosting;

namespace OpenCup.Server;

/// <summary>
/// Servidor que sirve la interfaz de openCUP (index.html, src/ y assets/)
/// y expone una API mínima. Es el equivalente en C#/.NET de server.mjs.
/// </summary>
public static class ServerApp
{
    /// <summary>
    /// Crea el servidor en el puerto indicado.
    /// </summary>
    public static WebApplication Create(string[] args, string contentRoot, int port)
    {
        var options = new WebApplicationOptions
        {
            Args = args,
            ContentRootPath = contentRoot,
        };

        var builder = WebApplication.CreateBuilder(options);
        builder.WebHost.UseUrls($"http://127.0.0.1:{port}");
        builder.Logging.SetMinimumLevel(LogLevel.Warning);

        var app = builder.Build();

        var provider = new PhysicalFileProvider(contentRoot);
        app.UseDefaultFiles(new DefaultFilesOptions { FileProvider = provider });
        app.UseStaticFiles(new StaticFileOptions { FileProvider = provider });

        app.MapGet("/api/health", () => Results.Json(new
        {
            ok = true,
            runtime = $".NET {Environment.Version}",
            port,
        }));

        app.MapGet("/api/version", () => Results.Json(new { name = "openCUP", version = "0.4.2", backend = "csharp" }));

        // La importación de borradores de CapCut sigue en el servidor Node hasta su porte.
        app.MapGet("/api/capcut/roots", () => Results.Json(new
        {
            ok = false,
            error = "Pendiente de portar a C#. Arranca server.mjs para la importación de CapCut.",
            roots = Array.Empty<string>(),
        }));

        return app;
    }

    /// <summary>
    /// Arranca en el primer puerto libre del rango indicado.
    /// </summary>
    public static (WebApplication app, int port)? Start(string[] args, string contentRoot, int firstPort, int lastPort)
    {
        for (var port = firstPort; port <= lastPort; port++)
        {
            var app = Create(args, contentRoot, port);
            try
            {
                app.StartAsync().GetAwaiter().GetResult();
                return (app, port);
            }
            catch
            {
                try { app.DisposeAsync().AsTask().GetAwaiter().GetResult(); } catch { /* ignorar */ }
            }
        }
        return null;
    }
}
