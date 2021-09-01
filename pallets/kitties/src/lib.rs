#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::traits::ExistenceRequirement;
    use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::Randomness};

    use codec::{Decode, Encode};
    use frame_system::pallet_prelude::*;
    use sp_io::hashing::blake2_128;
    use sp_runtime::traits::{AtLeast32BitUnsigned, Bounded, One};
    use sp_std::prelude::*;

    use frame_support::traits::Currency;
    use frame_support::traits::ReservableCurrency;

    #[derive(Encode, Decode)]
    pub struct Kitty(pub [u8; 16]);

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Randomness: Randomness<Self::Hash, Self::BlockNumber>;

        // 定义 KittyIndex 类型，要求实现指定的 trait
        type KittyIndex: Parameter + AtLeast32BitUnsigned + Default + Copy + Bounded;

        // 创建Kitty需要质押数量
        type KittyReserve: Get<BalanceOf<Self>>;

        // Currency 类型，用于质押等于资产相关的操作
        type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
    }

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        KittyCreate(T::AccountId, T::KittyIndex),
        KittyTransfer(T::AccountId, T::AccountId, T::KittyIndex),
        KittyForSale(T::AccountId, T::KittyIndex, Option<BalanceOf<T>>),
        KittySaleOut(T::AccountId, T::KittyIndex, Option<BalanceOf<T>>),
    }

    #[pallet::error]
    pub enum Error<T> {
        KittiesCountOverflow,
        NotOwner,
        SameParentIndex,
        InvalidKittyIndex,
        MoneyNotEnough,
        AlreadyOwned,
        NotForSale,
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn kitties_count)]
    pub type KittiesCount<T: Config> = StorageValue<_, T::KittyIndex, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn kitties)]
    pub type Kitties<T: Config> =
        StorageMap<_, Blake2_128Concat, T::KittyIndex, Option<Kitty>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn owner)]
    pub type Owner<T: Config> =
        StorageMap<_, Blake2_128Concat, T::KittyIndex, Option<T::AccountId>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn kitty_prices)]
    pub type KittyPrices<T: Config> =
        StorageMap<_, Blake2_128Concat, T::KittyIndex, Option<BalanceOf<T>>, ValueQuery>;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(0)]
        pub fn create(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let kitty_id = Self::get_kitty_id()?;
            let dna = Self::random_value(&who);

            // 质押资产
            T::Currency::reserve(&who, T::KittyReserve::get())
                .map_err(|_| Error::<T>::MoneyNotEnough)?;

            Kitties::<T>::insert(kitty_id, Some(Kitty(dna)));
            Owner::<T>::insert(kitty_id, Some(who.clone()));
            KittiesCount::<T>::put(kitty_id + One::one());

            Self::deposit_event(Event::KittyCreate(who, kitty_id));
            Ok(())
        }

        #[pallet::weight(0)]
        pub fn transfer(
            origin: OriginFor<T>,
            new_owner: T::AccountId,
            kitty_id: T::KittyIndex,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                Some(who.clone()) != Some(new_owner.clone()),
                Error::<T>::AlreadyOwned
            );
            ensure!(
                Some(who.clone()) == Owner::<T>::get(kitty_id),
                Error::<T>::NotOwner
            );

            // 新拥有者质押资产
            T::Currency::reserve(&new_owner, T::KittyReserve::get())
                .map_err(|_| Error::<T>::MoneyNotEnough)?;
            // 解除原质押资产
            T::Currency::unreserve(&who, T::KittyReserve::get());

            Owner::<T>::insert(kitty_id, Some(new_owner.clone()));
            Self::deposit_event(Event::KittyTransfer(who, new_owner, kitty_id));
            Ok(())
        }

        #[pallet::weight(0)]
        pub fn bread(
            origin: OriginFor<T>,
            kitty_id_1: T::KittyIndex,
            kitty_id_2: T::KittyIndex,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let kitty_id = Self::get_kitty_id()?;

            ensure!(kitty_id_1 != kitty_id_2, Error::<T>::SameParentIndex);

            let kitty1 = Self::kitties(kitty_id_1).ok_or(Error::<T>::InvalidKittyIndex)?;
            let kitty2 = Self::kitties(kitty_id_2).ok_or(Error::<T>::InvalidKittyIndex)?;
            let dna_1 = kitty1.0;
            let dna_2 = kitty2.0;

            let selector = Self::random_value(&who);
            let mut new_dna = [0u8; 16];

            for i in 0..dna_1.len() {
                new_dna[i] = (selector[i] & dna_1[i]) | (!selector[i] & dna_2[i]);
            }

            // 质押资产
            T::Currency::reserve(&who, T::KittyReserve::get())
                .map_err(|_| Error::<T>::MoneyNotEnough)?;

            Kitties::<T>::insert(kitty_id, Some(Kitty(new_dna)));
            Owner::<T>::insert(kitty_id, Some(who.clone()));
            KittiesCount::<T>::put(kitty_id + One::one());
            Self::deposit_event(Event::KittyCreate(who, kitty_id));

            Ok(())
        }

        #[pallet::weight(0)]
        pub fn sale(
            origin: OriginFor<T>,
            kitty_id: T::KittyIndex,
            sale_price: Option<BalanceOf<T>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                Some(who.clone()) == Owner::<T>::get(kitty_id),
                Error::<T>::NotOwner
            );

            KittyPrices::<T>::insert(kitty_id, sale_price);

            Self::deposit_event(Event::KittyForSale(who, kitty_id, sale_price));
            Ok(())
        }

        #[pallet::weight(0)]
        pub fn buy(origin: OriginFor<T>, kitty_id: T::KittyIndex) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let kitty_owner = Owner::<T>::get(kitty_id).ok_or(Error::<T>::NotOwner)?;
            let kitty_price = KittyPrices::<T>::get(kitty_id).ok_or(Error::<T>::NotForSale)?;
            ensure!(
                Some(who.clone()) != Some(kitty_owner.clone()),
                Error::<T>::AlreadyOwned
            );

            // 新拥有者质押资产
            T::Currency::reserve(&who, T::KittyReserve::get())
                .map_err(|_| Error::<T>::MoneyNotEnough)?;

            //转账（购买）
            T::Currency::transfer(
                &who,
                &kitty_owner,
                kitty_price,
                ExistenceRequirement::KeepAlive,
            )?;

            // 解除原质押资产
            T::Currency::unreserve(&kitty_owner, T::KittyReserve::get());

            //更改拥有人
            Owner::<T>::insert(kitty_id, Some(who.clone()));

            //移除挂售
            KittyPrices::<T>::remove(kitty_id);

            Self::deposit_event(Event::KittySaleOut(who, kitty_id, Some(kitty_price)));
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        // 获取当前Kitty_id (从0开始)
        fn get_kitty_id() -> sp_std::result::Result<T::KittyIndex, DispatchError> {
            let kitty_id = Self::kitties_count();
            if kitty_id == T::KittyIndex::max_value() {
                return Err(Error::<T>::KittiesCountOverflow.into());
            }
            Ok(kitty_id)
        }

        fn random_value(sender: &T::AccountId) -> [u8; 16] {
            let payload = (
                T::Randomness::random_seed(), // 通过最近区块信息生成的随机数种子
                &sender,
                <frame_system::Pallet<T>>::extrinsic_index(), // 当前交易在区块中的顺序
            );
            payload.using_encoded(blake2_128)
        }
    }
}
