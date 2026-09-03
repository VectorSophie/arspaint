using Pinta.Core;
using Pinta.Core.Ars;

namespace Pinta.Ars;

/// <summary>
/// The first slice of ARSPaint's direct tool-switching keyboard layer
/// (spec section 21). Three of the five bindings there - b/e/f for brush/
/// eraser/fill - already exist as Pinta's own tool ShortcutKeys and need no
/// changes. This adds the two that don't: i for the color picker (Pinta
/// binds K instead) and v for rectangle select (Pinta binds S, shared with
/// lasso/magic wand). Registered as ARS commands rather than reassigning
/// Pinta's own tool shortcuts, so existing muscle memory (K, S) keeps
/// working unchanged - see docs/architecture.md.
///
/// Also adds Tab as a canvas-only mode toggle (spec section 31): hides the
/// toolbar, tool palette, and side docks, keeping the canvas and status bar.
/// </summary>
public static class ArsKeyboardLayer
{
	private static bool initialized;
	private static bool canvas_only_active;
	private static bool saved_tool_bar, saved_tool_box, saved_tool_windows;

	public static void Initialize ()
	{
		if (initialized)
			return;
		initialized = true;

		ArsCommandRegistry.Register (new ArsCommandEntry (
			"ars-tool-color-picker",
			Translations.GetString ("Color Picker"),
			new Gdk.Key (Gdk.Constants.KEY_I),
			() => PintaCore.Tools.SetCurrentTool ("ColorPickerTool")));

		ArsCommandRegistry.Register (new ArsCommandEntry (
			"ars-tool-rectangle-select",
			Translations.GetString ("Rectangle Select"),
			new Gdk.Key (Gdk.Constants.KEY_V),
			() => PintaCore.Tools.SetCurrentTool ("RectangleSelectTool")));

		ArsCommandRegistry.Register (new ArsCommandEntry (
			"ars-view-canvas-only",
			Translations.GetString ("Toggle Canvas-Only Mode"),
			new Gdk.Key (Gdk.Constants.KEY_Tab),
			ToggleCanvasOnly));
	}

	private static void ToggleCanvasOnly ()
	{
		ViewActions view = PintaCore.Actions.View;

		if (!canvas_only_active) {
			saved_tool_bar = view.ToolBar.Value;
			saved_tool_box = view.ToolBox.Value;
			saved_tool_windows = view.ToolWindows.Value;

			view.ToolBar.Value = false;
			view.ToolBox.Value = false;
			view.ToolWindows.Value = false;
		} else {
			view.ToolBar.Value = saved_tool_bar;
			view.ToolBox.Value = saved_tool_box;
			view.ToolWindows.Value = saved_tool_windows;
		}

		canvas_only_active = !canvas_only_active;
	}
}
