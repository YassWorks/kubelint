/// A container image reference split into its parts, such as
/// `registry.example.com:5000/team/app:1.2@sha256:...`.
///
/// The split follows the reference grammar of `github.com/distribution/reference`
/// (`reference.go`): the digest follows the first `@`, and the tag follows the last `:`
/// after the last `/`, so a registry port is never read as a tag. The parts are not
/// validated.
///
/// ```
/// use kubelint_k8s::ImageRef;
///
/// let image = ImageRef::parse("localhost:5000/app");
/// assert_eq!((image.name, image.tag, image.digest), ("localhost:5000/app", None, None));
/// let image = ImageRef::parse("nginx:1.27@sha256:abc");
/// assert_eq!((image.name, image.tag, image.digest), ("nginx", Some("1.27"), Some("sha256:abc")));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageRef<'a> {
    /// The repository, with its registry and port when present.
    pub name: &'a str,
    pub tag: Option<&'a str>,
    pub digest: Option<&'a str>,
}

impl<'a> ImageRef<'a> {
    pub fn parse(image: &'a str) -> Self {
        let (rest, digest) = match image.split_once('@') {
            Some((rest, digest)) => (rest, Some(digest)),
            None => (image, None),
        };
        let last_part = rest.rfind('/').map_or(0, |slash| slash + 1);
        let (name, tag) = match rest[last_part..].rfind(':') {
            Some(colon) => {
                let colon = last_part + colon;
                (&rest[..colon], Some(&rest[colon + 1..]))
            }
            None => (rest, None),
        };
        Self { name, tag, digest }
    }

    /// The repository path of [`Self::name`], without the registry host. The first
    /// part is a host when it contains `.` or `:`, is `localhost` or has an uppercase
    /// letter (`splitDockerDomain` in `normalize.go` of `distribution/reference`).
    ///
    /// ```
    /// use kubelint_k8s::ImageRef;
    ///
    /// assert_eq!(ImageRef::parse("ghcr.io/org/app:1").path(), "org/app");
    /// assert_eq!(ImageRef::parse("org/app").path(), "org/app");
    /// ```
    pub fn path(self) -> &'a str {
        match self.name.split_once('/') {
            Some((host, path))
                if host.contains(['.', ':'])
                    || host == "localhost"
                    || host.bytes().any(|b| b.is_ascii_uppercase()) =>
            {
                path
            }
            _ => self.name,
        }
    }
}
