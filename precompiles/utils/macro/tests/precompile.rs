// Copyright 2019-2022 PureStake Inc.
// Copyright 2025 3Dpass
// This file is part of 3Dpass.

// 3Dpass is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// 3Dpass is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with 3Dpass. If not, see <http://www.gnu.org/licenses/>.

#[test]
fn ui() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/precompile/compile-fail/**/*.rs");
	t.pass("tests/precompile/pass/**/*.rs");
}

#[test]
fn expand() {
	macrotest::expand_without_refresh("tests/precompile/expand/**/*.rs");
}
