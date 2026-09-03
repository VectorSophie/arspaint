using Pinta.Core;
using Pinta.Core.Ars;

namespace Pinta.Ars;

/// <summary>
/// Commands for assigning a semantic <see cref="LayerRole"/> to the current
/// layer, wired into the layers panel's context menu. Static, like the rest
/// of the ARS action surface, to match the PintaCore.* singleton-manager
/// convention already used throughout the app. See docs/architecture.md.
/// </summary>
public static class LayerRoleActions
{
	public static Command MarkGeneric { get; } = new ("ars-role-generic", Translations.GetString ("Generic"), null, null);
	public static Command MarkReference { get; } = new ("ars-role-reference", Translations.GetString ("Reference"), null, null);
	public static Command MarkInk { get; } = new ("ars-role-ink", Translations.GetString ("Ink"), null, null);
	public static Command MarkColor { get; } = new ("ars-role-color", Translations.GetString ("Color"), null, null);
	public static Command MarkShade { get; } = new ("ars-role-shade", Translations.GetString ("Shade"), null, null);

	private static bool handlers_registered;

	public static void RegisterActions (Gtk.Application app)
	{
		app.AddCommands ([MarkGeneric, MarkReference, MarkInk, MarkColor, MarkShade]);
	}

	public static void RegisterHandlers ()
	{
		if (handlers_registered)
			return;
		handlers_registered = true;

		MarkGeneric.Activated += (_, _) => SetRole (LayerRole.Generic);
		MarkReference.Activated += (_, _) => SetRole (LayerRole.Reference);
		MarkInk.Activated += (_, _) => SetRole (LayerRole.Ink);
		MarkColor.Activated += (_, _) => SetRole (LayerRole.Color);
		MarkShade.Activated += (_, _) => SetRole (LayerRole.Shade);
	}

	/// <summary>
	/// Appends a "Layer Role" submenu with all five options to an existing
	/// context/menu model, e.g. the layers panel's right-click menu.
	/// </summary>
	public static void AppendRoleSubmenu (Gio.Menu menu)
	{
		Gio.Menu roleMenu = Gio.Menu.New ();
		roleMenu.AppendItem (MarkGeneric.CreateMenuItem ());
		roleMenu.AppendItem (MarkReference.CreateMenuItem ());
		roleMenu.AppendItem (MarkInk.CreateMenuItem ());
		roleMenu.AppendItem (MarkColor.CreateMenuItem ());
		roleMenu.AppendItem (MarkShade.CreateMenuItem ());

		menu.AppendSubmenu (Translations.GetString ("Layer Role"), roleMenu);
	}

	private static void SetRole (LayerRole role)
	{
		if (!PintaCore.Workspace.HasOpenDocuments)
			return;

		Document doc = PintaCore.Workspace.ActiveDocument;
		UserLayer layer = doc.Layers.CurrentUserLayer;
		LayerRole oldRole = LayerRoleStore.Get (layer);

		if (oldRole == role)
			return;

		LayerRoleStore.Set (layer, role);

		doc.History.PushNewItem (new SetLayerRoleHistoryItem (
			doc.Layers.CurrentUserLayerIndex,
			oldRole,
			role));
	}
}
