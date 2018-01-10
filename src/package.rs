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

use std::collections::BTreeMap as Map;

use version::VersionBuf;

#[derive(Copy,Clone,Debug,Eq,PartialEq)]
pub enum Constraint {
	Equal,
	Greater,
	GreaterEqual,
	Less,
	LessEqual,
}

#[derive(Clone,Debug,Eq,PartialEq)]
pub struct VersionedTarget {
	pub name: String,
	pub version: VersionBuf,
}

#[derive(Clone,Debug,Eq,PartialEq)]
pub struct VersionConstraint {
	pub version:    VersionBuf,
	pub constraint: Constraint,
}

#[derive(Clone,Debug,Eq,PartialEq)]
pub struct Package {
	pub name:          String,
	pub version:       VersionBuf,

	pub url:           Option<String>,
	pub description:   Option<String>,
	pub licenses:      Vec<String>,

	pub groups:        Vec<String>,
	pub backup:        Vec<String>,

	pub provides:      Map<String, Option<VersionBuf>>,
	pub conflicts:     Map<String, Option<VersionConstraint>>,
	pub replaces:      Map<String, Option<VersionConstraint>>,

	pub depends:       Map<String, Option<VersionConstraint>>,
	pub opt_depends:   Map<String, Option<VersionConstraint>>,
	pub make_depends:  Map<String, Option<VersionConstraint>>,
	pub check_depends: Map<String, Option<VersionConstraint>>,
}

/// A partial package with some information possibly missing.
#[derive(Default,Clone,Debug,Eq,PartialEq)]
pub struct PartialPackage {
	pub name:          Option<String>,
	pub version:       Option<VersionBuf>,

	pub url:           Option<String>,
	pub description:   Option<String>,
	pub licenses:      Option<Vec<String>>,

	pub groups:        Option<Vec<String>>,
	pub backup:        Option<Vec<String>>,

	pub provides:      Option<Map<String, Option<VersionBuf>>>,
	pub conflicts:     Option<Map<String, Option<VersionConstraint>>>,
	pub replaces:      Option<Map<String, Option<VersionConstraint>>>,

	pub depends:       Option<Map<String, Option<VersionConstraint>>>,
	pub opt_depends:   Option<Map<String, Option<VersionConstraint>>>,
	pub make_depends:  Option<Map<String, Option<VersionConstraint>>>,
	pub check_depends: Option<Map<String, Option<VersionConstraint>>>,
}

impl PartialPackage {
	/// Try to create a package from the partial package.
	pub fn into_package(self) -> Result<Package, String> {
		println!("{:?}", self);
		Ok(Package {
			name:    self.name.ok_or_else(||    String::from("missing pkgname"))?,
			version: self.version.ok_or_else(|| String::from("missing pkgver"))?,

			url:           self.url,
			description:   self.description,
			licenses:      self.licenses.unwrap_or_default(),

			groups:        self.groups.unwrap_or_default(),
			backup:        self.backup.unwrap_or_default(),

			provides:      self.provides.unwrap_or_default(),
			conflicts:     self.conflicts.unwrap_or_default(),
			replaces:      self.replaces.unwrap_or_default(),

			depends:       self.depends.unwrap_or_default(),
			opt_depends:   self.opt_depends.unwrap_or_default(),
			make_depends:  self.make_depends.unwrap_or_default(),
			check_depends: self.check_depends.unwrap_or_default(),
		})
	}

	pub fn add_base(&mut self, base: &PartialPackage) {
		if self.url.is_none()         { self.url         = base.url.clone()         }
		if self.description.is_none() { self.description = base.description.clone() }
		if self.licenses.is_none()    { self.licenses    = base.licenses.clone()    }

		if self.groups.is_none()      { self.groups      = base.groups.clone()      }
		if self.backup.is_none()      { self.backup      = base.backup.clone()      }

		if self.provides.is_none()    { self.provides    = base.provides.clone()    }
		if self.conflicts.is_none()   { self.conflicts   = base.conflicts.clone()   }
		if self.replaces.is_none()    { self.replaces    = base.replaces.clone()    }

		if self.depends.is_none()     { self.depends     = base.depends.clone()     }
		if self.opt_depends.is_none() { self.opt_depends = base.opt_depends.clone() }
	}
}
