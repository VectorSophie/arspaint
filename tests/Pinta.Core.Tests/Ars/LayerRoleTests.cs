using Cairo;
using NUnit.Framework;
using Pinta.Core.Ars;

namespace Pinta.Core.Tests.Ars;

[TestFixture]
internal sealed class LayerRoleTests
{
	// Forces Utilities' static constructor (Cairo/Gio/Gdk module init) to run
	// before any test in this fixture touches Cairo - otherwise whether that
	// already happened depends on unrelated fixture run order.
	static LayerRoleTests () => _ = Utilities.GetAssetPath (string.Empty);

	private static UserLayer NewLayer ()
		=> new (CairoExtensions.CreateImageSurface (Format.Argb32, 1, 1));

	[Test]
	public void UnsetLayerDefaultsToGeneric ()
	{
		UserLayer layer = NewLayer ();
		Assert.That (LayerRoleStore.Get (layer), Is.EqualTo (LayerRole.Generic));
	}

	[Test]
	public void SetThenGetRoundTrips ()
	{
		UserLayer layer = NewLayer ();
		LayerRoleStore.Set (layer, LayerRole.Ink);
		Assert.That (LayerRoleStore.Get (layer), Is.EqualTo (LayerRole.Ink));
	}

	[Test]
	public void SettingBackToGenericForgetsTheLayer ()
	{
		UserLayer layer = NewLayer ();
		LayerRoleStore.Set (layer, LayerRole.Reference);
		LayerRoleStore.Set (layer, LayerRole.Generic);
		Assert.That (LayerRoleStore.Get (layer), Is.EqualTo (LayerRole.Generic));
	}

	[Test]
	public void RolesAreIndependentPerLayer ()
	{
		UserLayer a = NewLayer ();
		UserLayer b = NewLayer ();
		LayerRoleStore.Set (a, LayerRole.Color);
		Assert.That (LayerRoleStore.Get (a), Is.EqualTo (LayerRole.Color));
		Assert.That (LayerRoleStore.Get (b), Is.EqualTo (LayerRole.Generic));
	}

	[TestCase (LayerRole.Generic, null)]
	[TestCase (LayerRole.Reference, "[REF]")]
	[TestCase (LayerRole.Ink, "[INK]")]
	[TestCase (LayerRole.Color, "[COL]")]
	[TestCase (LayerRole.Shade, "[SHD]")]
	public void BadgeMatchesRole (LayerRole role, string? expected)
		=> Assert.That (role.Badge (), Is.EqualTo (expected));

	[TestCase (LayerRole.Generic)]
	[TestCase (LayerRole.Reference)]
	[TestCase (LayerRole.Ink)]
	[TestCase (LayerRole.Color)]
	[TestCase (LayerRole.Shade)]
	public void OraAttributeRoundTrips (LayerRole role)
	{
		string? attribute = role.OraAttributeValue ();
		LayerRole parsed = LayerRoleExtensions.ParseOraAttributeValue (attribute);
		Assert.That (parsed, Is.EqualTo (role));
	}

	[Test]
	public void GenericRoleWritesNoOraAttribute ()
		=> Assert.That (LayerRole.Generic.OraAttributeValue (), Is.Null);

	[TestCase (null)]
	[TestCase ("")]
	[TestCase ("not-a-real-role")]
	public void UnknownOraAttributeValueParsesAsGeneric (string? value)
		=> Assert.That (LayerRoleExtensions.ParseOraAttributeValue (value), Is.EqualTo (LayerRole.Generic));
}
