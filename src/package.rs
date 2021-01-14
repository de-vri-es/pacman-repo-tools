// Copyright (c) 2017, Maarten de Vries
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
// * Redistributions of source code must retain the above copyright notice, this
//   list of conditions and the following disclaimer.
//
// * Redistributions in binary form must reproduce the above copyright notice,
//   this list of conditions and the following disclaimer in the documentation
//   and/or other materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use std::collections::BTreeMap;

use crate::version::Version;

type DependencyMap = BTreeMap<String, Option<VersionConstraint>>;
type ProvidesMap = BTreeMap<String, Option<Version>>;

/// Metadata about a pacman package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Package {
	pub pkgname: String,
	pub version: Version,

	pub url: Option<String>,
	pub description: Option<String>,
	pub licenses: Vec<String>,

	pub groups: Vec<String>,
	pub backup: Vec<String>,

	pub provides: ProvidesMap,
	pub conflicts: DependencyMap,
	pub replaces: DependencyMap,

	pub depends: DependencyMap,
	pub opt_depends: DependencyMap,
	pub make_depends: DependencyMap,
	pub check_depends: DependencyMap,
}

/// A version constraint operator.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Constraint {
	Equal,
	Greater,
	GreaterEqual,
	Less,
	LessEqual,
}

/// A version constraint as used for package dependencies.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VersionConstraint {
	pub version: Version,
	pub constraint: Constraint,
}
