Name:           wallpaper-composer
Version:        0.1.0
Release:        1%{?dist}
Summary:        Dynamic Linux wallpaper composer (Rust)

License:        GPL-3.0-or-later
URL:            https://github.com/<org-or-user>/wallpaper-composer
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust
BuildRequires:  cargo

%description
Wallpaper Composer renders dynamic wallpapers using local or public image/quote sources,
clock overlays, and desktop backend integration.

%prep
%autosetup -n %{name}-%{version}

%build
cargo build --release -p wc-cli

%install
install -Dpm0755 target/release/wc-cli %{buildroot}%{_bindir}/wc-cli
install -Dpm0644 README.md %{buildroot}%{_docdir}/%{name}/README.md
install -Dpm0644 LICENSE %{buildroot}%{_licensedir}/%{name}/LICENSE

%files
%license %{_licensedir}/%{name}/LICENSE
%doc %{_docdir}/%{name}/README.md
%{_bindir}/wc-cli

%changelog
* Wed Mar 04 2026 Wallpaper Composer Contributors <opensource@example.com> - 0.1.0-1
- Initial RPM packaging skeleton.

