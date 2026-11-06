//! The ten manifests, embedded.
//!
//! `include_str!` rather than a build script or a serde load at launch: the
//! bytes the crate reports are the bytes that were checked in, and the harness
//! that reads them does not have to start the application. A release descriptor
//! can bind these to a binary later; that is a packaging step, not a source one.
//!
//! The table describes the surface this crate re-exports, which is why it lives
//! here rather than in the binary that prints it: a consumer asking what the
//! workspace ships is asking about the items it just imported. The binary keeps
//! its own path to the same values (`Agito::All`) by re-exporting this module,
//! so there is one definition and no second answer.
//!
//! The facade itself is not one of the ten. It provides no port, so it has
//! nothing a harness would prepare for and nothing `agito verify --module`
//! could answer; its own `module.json` declares the `facade` kind and the ports
//! it re-exports, which is a statement about this table rather than an entry in
//! it.

/// One module, as the workspace ships it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Manifest {
	/// The manifest's own name, which is the logical identity across the
	/// migration.
	pub Name:&'static str,
	/// The kind of thing it is: adapter, capability, runtime, workflow,
	/// composition.
	pub Kind:&'static str,
	/// The port it provides.
	pub Port:&'static str,
	/// The verbs it declares under `Provide.Verb`, in its own order.
	///
	/// A verb is an extra thing the module can be asked to do beyond
	/// answering its port - the chat driver's `agito.chat.note` is the
	/// one the workspace ships - and most modules declare none, so an empty
	/// slice is the ordinary case rather than a missing value.
	pub Verbs:&'static [&'static str],
	/// The retention class the manifest declares.
	pub State:&'static str,
	/// The manifest's bytes, exactly as checked in.
	pub JSON:&'static str,
}

/// Every module this workspace ships, in dependency order.
pub const All:[Manifest; 10] = [
	Manifest {
		Name:"@agito/runtime",
		Kind:"RunTime",
		Port:"agito.runtime",
		Verbs:&[],
		State:"HoldWorkspace",
		JSON:include_str!("../../../RunTime/module.json"),
	},
	// The port shape a third-party substrate implements. Kind `capability` and
	// not `adapter`: the boundary rule lets an adapter name a capability's port
	// shapes, which is the edge a third-party adapter draws when it depends on
	// this crate, while an adapter naming another adapter is the edge the rule
	// forbids. Its manifest names no errors, because a code belongs to the
	// adapter that reports it rather than to the shape it implements.
	Manifest {
		Name:"@agito/adapter",
		Kind:"Capability",
		Port:"agito.adapter",
		Verbs:&[],
		State:"Stateless",
		JSON:include_str!("../../../Adapter/module.json"),
	},
	// The template corpus as data. It depends on the runtime and nothing else:
	// the round trip leaves through a `Poster` this crate declares and never
	// implements, so its manifest says `network: false` and means it.
	Manifest {
		Name:"@agito/lane",
		Kind:"Capability",
		Port:"agito.lane.decide",
		Verbs:&[],
		State:"Stateless",
		JSON:include_str!("../../../Lane/module.json"),
	},
	Manifest {
		Name:"@agito/spacetime-catalog",
		Kind:"Capability",
		Port:"agito.catalog.rules",
		Verbs:&[],
		State:"Stateless",
		JSON:include_str!("../../../Spacetime/Catalog/module.json"),
	},
	Manifest {
		Name:"@agito/skill-contract",
		Kind:"Capability",
		Port:"agito.skill.validate",
		Verbs:&[],
		State:"Stateless",
		JSON:include_str!("../../../Skill/Contract/module.json"),
	},
	// The one module that declares a verb. `agito.chat.note` is not the
	// port: a caller asks `agito.chat.ask` and gets an answer, while `note`
	// is the call that prints a research note built from that same answer.
	// It carries its own (empty) error list, so the verb is a statement a
	// harness can read rather than a name nobody lists.
	Manifest {
		Name:"@agito/chat-driver",
		Kind:"Capability",
		Port:"agito.chat.ask",
		Verbs:&["agito.chat.note"],
		State:"HoldTransport",
		JSON:include_str!("../../../Chat/Driver/module.json"),
	},
	Manifest {
		Name:"@agito/browser-cdp",
		Kind:"Adapter",
		Port:"agito.browse",
		Verbs:&[],
		State:"HoldTransport",
		JSON:include_str!("../../../Browser/CDP/module.json"),
	},
	Manifest {
		Name:"@agito/design-verification",
		Kind:"Capability",
		Port:"agito.verify.design",
		Verbs:&[],
		State:"Stateless",
		JSON:include_str!("../../../Design/Verification/module.json"),
	},
	Manifest {
		Name:"@agito/workflow",
		Kind:"Workflow",
		Port:"agito.workflow",
		Verbs:&[],
		State:"Stateless",
		JSON:include_str!("../../../Workflow/module.json"),
	},
	Manifest {
		Name:"@agito/composition",
		Kind:"Composition",
		Port:"agito.compose",
		Verbs:&[],
		State:"Stateless",
		JSON:include_str!("../../../Agito/module.json"),
	},
];

/// The module with this name, if the workspace ships it.
pub fn Find(Name:&str) -> Option<&'static Manifest> { All.iter().find(|Manifest| Manifest.Name == Name) }
