use support::{decl_event, decl_module, decl_storage, StorageMap, StorageValue, ensure};
use system::ensure_signed;
use parity_codec::{Decode, Encode};
use core::u32::MAX as MAX_SUBJECT;

pub trait Trait: system::Trait + timestamp::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

pub type Subject = u32;

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Clone, Default, PartialEq)]
pub struct Credential<Timestamp, AccountId> {
   subject: Subject,
   when: Timestamp,
   by: AccountId
}

decl_storage! {
    trait Store for Module<T: Trait> as VerifiableCreds {
        // global nonce for subject count
        SubjectCount get(subject_count) config(): Subject;
        // Issuers can issue credentials to others.
        // Issuer to Subject mapping.
        Subjects get(subjects) config(): map Subject => T::AccountId;
        // Credentials store.
        // Mapping (holder, subject) to Credential.
        Credentials get(credentials): map (T::AccountId, Subject) => Credential<T::Moment, T::AccountId>;
    }
    extra_genesis_skip_phantom_data_field;
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        // A credential is issued - holder, subj, issuer
        CredentialIssued(AccountId, Subject, AccountId),
        // A credential is revoked - holder, subj, issuer
        CredentialRevoked(AccountId, Subject, AccountId),
        // A new subject is created.
        SubjectCreated(AccountId, Subject),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        /// Issue a credential to an identity.
        /// Only an issuer can call this function.
        pub fn issue_credential(origin, to: T::AccountId, subject: Subject) {
            // Check if origin is an issuer.
            // Issue the credential - add to storage.

            let sender = ensure_signed(origin)?;
            let subject_issuer = Self::subjects(subject);
            ensure!(subject_issuer == sender, "Unauthorized.");

            let now = <timestamp::Module<T>>::get();
            let cred = Credential {
              subject,
              when: now,
              by: sender.clone()
            };

            <Credentials<T>>::insert((to.clone(), subject), cred);

            Self::deposit_event(RawEvent::CredentialIssued(to, subject, sender));
        }

        /// Revoke a credential.
        /// Only an issuer can call this function. 
        pub fn revoke_credential(origin, to: T::AccountId, subject: Subject) {
            // Check if origin is an issuer.
            // Check if credential is issued.
            // Change the bool flag of the stored credential tuple to false.

            let sender = ensure_signed(origin)?;
            let subject_issuer = Self::subjects(subject);
            ensure!(subject_issuer == sender, "Unauthorized.");
            ensure!(<Credentials<T>>::exists((to.clone(), subject)), "Credential not issued yet.");

            <Credentials<T>>::remove((to.clone(), subject));
            Self::deposit_event(RawEvent::CredentialRevoked(to, subject, sender));
        }

        /// Verify a credential.
        pub fn verify_credential(origin, holder: T::AccountId, subject: Subject) {
            let _sender = ensure_signed(origin)?;

            // Ensure credential is issued and allowed to be verified.
            ensure!(<Credentials<T>>::exists((holder.clone(), subject)), "Credential not issued yet.");
        }

        /// Create a new subject.
        pub fn create_subject(origin) {
            let sender = ensure_signed(origin)?;
            let subject_count = <SubjectCount<T>>::get();

            ensure!(subject_count < MAX_SUBJECT, "Max issuance count reached");

            <Subjects<T>>::insert(subject_count, sender.clone());

            // Update the subject nonce.
            <SubjectCount<T>>::put(subject_count + 1);

            // Deposit the event.
            Self::deposit_event(RawEvent::SubjectCreated(sender, subject_count));
        }
    }
}

#[cfg(test)]
mod tests {
  use super::*;

  use primitives::{Blake2Hasher, H256};
  use runtime_io::{with_externalities, TestExternalities};
  use runtime_primitives::{
    testing::{Digest, DigestItem, Header},
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
  };
  use support::{assert_noop, assert_ok, impl_outer_origin};

  impl_outer_origin! {
    pub enum Origin for VerifiableCredsTest {}
  }

  // For testing the module, we construct a mock runtime. This means
  // first constructing a configuration type (`Test`) which implements each of the
  // configuration traits of modules we use.
  #[derive(Clone, Eq, PartialEq)]
  pub struct VerifiableCredsTest;
  impl system::Trait for VerifiableCredsTest {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Digest = Digest;
    type AccountId = u64;
    type Lookup = IdentityLookup<u64>;
    type Header = Header;
    type Event = ();
    type Log = DigestItem;
  }
  impl timestamp::Trait for VerifiableCredsTest {
    type Moment = u64;
    type OnTimestampSet = ();
  }
  impl Trait for VerifiableCredsTest {
    type Event = ();
  }
  type VerifiableCreds = Module<VerifiableCredsTest>;

  // builds the genesis config store and sets mock values
  fn build_ext() -> TestExternalities<Blake2Hasher> {
    let mut t = system::GenesisConfig::<Test>::default()
      .build_storage()
      .unwrap()
      .0;
    t.extend(
      GenesisConfig::<Test> {
        subjects: vec![(1, 1), (2, 2)],
        subject_count: 3,
      }
      .build_storage()
      .unwrap()
      .0,
    );
    t.into()
  }

  
  #[test]
  fn should_fail_issue() {
    with_externalities(&mut new_test_ext(), || {
        assert_noop!(
            VerifiableCreds::issue_credential(Origin::signed(1), 3, 2),
            "Unauthorized.");
    });
  }

  #[test]
  fn should_issue() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(
            VerifiableCreds::issue_credential(Origin::signed(1), 3, 1));
    });
  }

  #[test]
  fn should_revoke() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(
            VerifiableCreds::issue_credential(Origin::signed(1), 3, 1));
        assert_ok!(
            VerifiableCreds::revoke_credential(Origin::signed(1), 3, 1));
    });
  }

  #[test]
  fn should_add_subject() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(
            VerifiableCreds::create_subject(Origin::signed(3)));
        assert_eq!(
            VerifiableCreds::subjects(3), 3);
    });
  }
}