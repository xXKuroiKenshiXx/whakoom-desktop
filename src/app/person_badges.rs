use super::*;
impl App {
    pub(super) fn person_pro(&self, username: &str) -> bool {
        self.library
            .account
            .iter()
            .chain(self.library.friends.iter())
            .chain(self.library.followers.iter())
            .chain(self.library.favorite_people.values())
            .chain(self.found_users.iter())
            .chain(self.profile.iter())
            .any(|u| u.username.eq_ignore_ascii_case(username) && u.pro)
    }
    pub(super) fn cache_person_pro(&mut self, user: &social::User) {
        let mut changed = false;
        for cached in self
            .library
            .friends
            .iter_mut()
            .chain(self.library.followers.iter_mut())
            .chain(self.library.favorite_people.values_mut())
            .chain(self.found_users.iter_mut())
        {
            if cached.username.eq_ignore_ascii_case(&user.username) && cached.pro != user.pro {
                cached.pro = user.pro;
                changed = true;
            }
        }
        if changed {
            self.save_library();
        }
    }
}
