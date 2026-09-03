using NUnit.Framework;
using Pinta.Core.Ars;

namespace Pinta.Core.Tests.Ars;

[TestFixture]
internal sealed class ArsCommandRegistryTests
{
	// Forces Utilities' static constructor (Cairo/Gio/Gdk module init) to run
	// before any test in this fixture touches Gdk - see the same fix in
	// LayerRoleTests for why this can't be left to fixture run order.
	static ArsCommandRegistryTests () => _ = Utilities.GetAssetPath (string.Empty);

	[Test]
	public void DispatchingABoundKeyRunsItsCommandAndReturnsTrue ()
	{
		bool ran = false;
		ArsCommandRegistry.Register (new ArsCommandEntry (
			"test-dispatch-runs",
			"test",
			new Gdk.Key (Gdk.Constants.KEY_F13),
			() => ran = true));

		bool dispatched = ArsCommandRegistry.TryDispatchKey (new Gdk.Key (Gdk.Constants.KEY_F13));

		Assert.That (dispatched, Is.True);
		Assert.That (ran, Is.True);
	}

	[Test]
	public void DispatchingAnUnboundKeyReturnsFalseAndRunsNothing ()
	{
		bool ran = false;
		ArsCommandRegistry.Register (new ArsCommandEntry (
			"test-dispatch-unbound",
			"test",
			new Gdk.Key (Gdk.Constants.KEY_F14),
			() => ran = true));

		bool dispatched = ArsCommandRegistry.TryDispatchKey (new Gdk.Key (Gdk.Constants.KEY_F15));

		Assert.That (dispatched, Is.False);
		Assert.That (ran, Is.False);
	}

	[Test]
	public void KeyMatchingIsCaseInsensitive ()
	{
		bool ran = false;
		ArsCommandRegistry.Register (new ArsCommandEntry (
			"test-dispatch-case",
			"test",
			new Gdk.Key (Gdk.Constants.KEY_F16),
			() => ran = true));

		// Gdk represents shifted/unshifted letters as different keyvals;
		// ToUpper() inside TryDispatchKey is what makes 'v' and 'V' bind to
		// the same command regardless of Shift state.
		bool dispatched = ArsCommandRegistry.TryDispatchKey (new Gdk.Key (Gdk.Constants.KEY_F16).ToUpper ());

		Assert.That (dispatched, Is.True);
		Assert.That (ran, Is.True);
	}

	[Test]
	public void RegisteredEntryAppearsInEntries ()
	{
		ArsCommandEntry entry = new (
			"test-entries-listing",
			"Shown in Entries",
			null,
			() => { });

		ArsCommandRegistry.Register (entry);

		Assert.That (ArsCommandRegistry.Entries, Does.Contain (entry));
	}
}
