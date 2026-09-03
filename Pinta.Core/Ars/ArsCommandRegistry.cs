using System;
using System.Collections.Generic;
using System.Linq;

namespace Pinta.Core.Ars;

/// <summary>
/// A single ARS command: enough metadata that keyboard dispatch today, and a
/// which-key popup / colon-command palette later, can all read from the same
/// list instead of maintaining their own separate tables (see
/// docs/architecture.md - "one command registry" is a spec requirement, not
/// just an implementation detail).
/// </summary>
public sealed record ArsCommandEntry (
	string Id,
	string Description,
	Gdk.Key? Key,
	Action Execute);

/// <summary>
/// Flat, process-wide list of ARS commands with an optional single-key
/// binding. Deliberately minimal - no counts, contexts, or repeatability
/// yet, since nothing in this codebase consumes those. Add them when a
/// which-key popup or colon-command mode actually needs them.
/// </summary>
public static class ArsCommandRegistry
{
	private static readonly List<ArsCommandEntry> entries = [];

	public static IReadOnlyList<ArsCommandEntry> Entries => entries;

	public static void Register (ArsCommandEntry entry) => entries.Add (entry);

	/// <summary>
	/// Runs the first registered command bound to this key, if any. Returns
	/// whether a command was found and run, so callers can fall through to
	/// their own handling otherwise.
	/// </summary>
	public static bool TryDispatchKey (Gdk.Key key)
	{
		ArsCommandEntry? entry = entries.FirstOrDefault (e => e.Key is { } k && k.ToUpper () == key.ToUpper ());

		if (entry is null)
			return false;

		entry.Execute ();
		return true;
	}
}
