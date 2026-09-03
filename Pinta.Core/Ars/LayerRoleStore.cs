using System;
using System.Runtime.CompilerServices;

namespace Pinta.Core.Ars;

/// <summary>
/// Holds the optional <see cref="LayerRole"/> for each <see cref="UserLayer"/>,
/// keyed by layer identity rather than by adding a field to Pinta's own
/// UserLayer class. Unset layers implicitly have <see cref="LayerRole.Generic"/>
/// and take no space here.
/// </summary>
public static class LayerRoleStore
{
	private static readonly ConditionalWeakTable<UserLayer, StrongBox<LayerRole>> roles = new ();

	public static event EventHandler<LayerRoleChangedEventArgs>? RoleChanged;

	public static LayerRole Get (UserLayer layer)
		=> roles.TryGetValue (layer, out var box) ? box.Value : LayerRole.Generic;

	public static void Set (UserLayer layer, LayerRole role)
	{
		if (Get (layer) == role)
			return;

		if (role == LayerRole.Generic)
			roles.Remove (layer);
		else
			roles.AddOrUpdate (layer, new StrongBox<LayerRole> (role));

		RoleChanged?.Invoke (null, new LayerRoleChangedEventArgs (layer, role));
	}
}

public sealed class LayerRoleChangedEventArgs (UserLayer layer, LayerRole role) : EventArgs
{
	public UserLayer Layer { get; } = layer;
	public LayerRole Role { get; } = role;
}
