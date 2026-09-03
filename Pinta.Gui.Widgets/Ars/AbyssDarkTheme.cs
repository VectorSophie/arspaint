using Gdk;
using Gtk;
using Pinta.Resources;

namespace Pinta.Ars;

/// <summary>
/// Swaps in the Abyss dark palette (style-abyss-dark.css) whenever
/// Adw.StyleManager reports dark mode, and removes it again in light mode -
/// GTK CSS has no media-query equivalent, so this has to be done in code by
/// reacting to the StyleManager's generic property-notify signal for "dark".
/// </summary>
public static class AbyssDarkTheme
{
	private static CssProvider? provider;
	private static bool initialized;

	public static void Initialize ()
	{
		if (initialized)
			return;
		initialized = true;

		Adw.StyleManager styleManager = Adw.StyleManager.GetDefault ();

		styleManager.OnNotify += (_, args) => {
			if (args.Pspec.GetName () == "dark")
				Apply (styleManager.Dark);
		};

		Apply (styleManager.Dark);
	}

	private static void Apply (bool dark)
	{
		Display? display = Display.GetDefault ();
		if (display is null)
			return;

		if (dark) {
			provider ??= ResourceLoader.LoadAbyssDarkCssProvider ();
			if (provider is not null)
				StyleContext.AddProviderForDisplay (display, provider, Gtk.Constants.STYLE_PROVIDER_PRIORITY_APPLICATION);
		} else if (provider is not null) {
			StyleContext.RemoveProviderForDisplay (display, provider);
		}
	}
}
