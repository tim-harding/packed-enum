use quote::format_ident;
use syn::Ident;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Orm<T> {
    o: T,
    r: T,
    m: T,
}

impl Orm<Ident> {
    pub fn from_ident(ident: Ident) -> Self {
        Self {
            r: format_ident!("{}Ref", ident),
            m: format_ident!("{}Mut", ident),
            o: ident,
        }
    }
}

impl<T> Orm<T> {
    pub const fn new(o: T, r: T, m: T) -> Self {
        Self { o, r, m }
    }

    pub fn as_ref(&self) -> Orm<&T> {
        Orm {
            o: &self.o,
            r: &self.r,
            m: &self.m,
        }
    }

    pub fn as_mut(&mut self) -> Orm<&mut T> {
        Orm {
            o: &mut self.o,
            r: &mut self.r,
            m: &mut self.m,
        }
    }

    pub fn into_tuple(self) -> (T, T, T) {
        let Self { o, r, m } = self;
        (o, r, m)
    }

    pub fn into_tuple_nest(self) -> (T, (T, T)) {
        let Self { o, r, m } = self;
        (o, (r, m))
    }
}

impl<T> From<(T, T, T)> for Orm<T> {
    fn from((a, b, c): (T, T, T)) -> Self {
        Self::new(a, b, c)
    }
}

impl<T> From<Orm<T>> for (T, T, T) {
    fn from(Orm { o, r, m }: Orm<T>) -> Self {
        (o, r, m)
    }
}

impl<T> FromIterator<Orm<T>> for Orm<Vec<T>> {
    fn from_iter<I: IntoIterator<Item = Orm<T>>>(iter: I) -> Self {
        let (o, (r, m)) = iter.into_iter().map(Orm::into_tuple_nest).unzip();
        Self { o, r, m }
    }
}

impl<T> Extend<Orm<T>> for Orm<Vec<T>> {
    fn extend<I: IntoIterator<Item = Orm<T>>>(&mut self, iter: I) {
        for item in iter.into_iter() {
            let Orm { o, r, m } = item;
            self.o.push(o);
            self.r.push(r);
            self.m.push(m);
        }
    }
}
