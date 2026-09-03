namespace Pinta.Core.Ars;

/// <summary>
/// Optional semantic role for a layer. A layer with a role is still a normal
/// Pinta layer in every other respect - the role only exists so ARSPaint can
/// make useful workflow decisions (reference export exclusion, smart
/// suggestions, workflow presets). See docs/architecture.md.
/// </summary>
public enum LayerRole
{
	Generic,
	Reference,
	Ink,
	Color,
	Shade,
}

public static class LayerRoleExtensions
{
	/// <summary>Short bracketed badge shown in the layers panel, e.g. "[REF]".</summary>
	public static string? Badge (this LayerRole role) => role switch {
		LayerRole.Reference => "[REF]",
		LayerRole.Ink => "[INK]",
		LayerRole.Color => "[COL]",
		LayerRole.Shade => "[SHD]",
		_ => null,
	};

	public static string DisplayName (this LayerRole role) => role switch {
		LayerRole.Reference => Translations.GetString ("Reference"),
		LayerRole.Ink => Translations.GetString ("Ink"),
		LayerRole.Color => Translations.GetString ("Color"),
		LayerRole.Shade => Translations.GetString ("Shade"),
		_ => Translations.GetString ("Generic"),
	};

	/// <summary>
	/// The .ora attribute value used to persist this role. Generic layers
	/// write no attribute at all, so an ordinary .ora file round-trips
	/// through ARSPaint with no visible metadata.
	/// </summary>
	public static string? OraAttributeValue (this LayerRole role) => role switch {
		LayerRole.Generic => null,
		_ => role.ToString ().ToLowerInvariant (),
	};

	public static LayerRole ParseOraAttributeValue (string? value) => value switch {
		"reference" => LayerRole.Reference,
		"ink" => LayerRole.Ink,
		"color" => LayerRole.Color,
		"shade" => LayerRole.Shade,
		_ => LayerRole.Generic,
	};
}
