namespace Pinta.Core.Ars;

public sealed class SetLayerRoleHistoryItem : BaseHistoryItem
{
	private readonly int layer_index;
	private readonly LayerRole old_role;
	private readonly LayerRole new_role;

	public SetLayerRoleHistoryItem (
		int layerIndex,
		LayerRole oldRole,
		LayerRole newRole)
		: base (Resources.Icons.LayerProperties, Translations.GetString ("Set Layer Role"))
	{
		layer_index = layerIndex;
		old_role = oldRole;
		new_role = newRole;
	}

	public override void Undo ()
		=> Apply (old_role);

	public override void Redo ()
		=> Apply (new_role);

	private void Apply (LayerRole role)
	{
		Document doc = PintaCore.Workspace.ActiveDocument;
		LayerRoleStore.Set (doc.Layers[layer_index], role);
	}
}
