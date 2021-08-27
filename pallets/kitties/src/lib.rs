#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::Randomness};
    // use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_io::hashing::blake2_128;
    use codec::{Encode, Decode};
    use sp_runtime::traits::StaticLookup;

    // #[derive(Encode, Decode)]
    // pub struct Kitty(pub [u8; 16]);

    type KittyIndex = u32;

    // emm owner还是从里面拿掉吧
    #[derive(Encode, Decode, Default, PartialEq, Clone)]
    pub struct Kitty<Balance> {
        price: Balance,
        id: KittyIndex,
        dna: [u8; 16],
        for_sale: bool
    }

    #[derive(Encode, Decode, Default, Clone)]
    pub struct MarketBidDetail<Balance, AccountID> {
        id: KittyIndex,
        price: Balance,
        who: AccountID
    }

    impl<Balance, AccountID> MarketBidDetail<Balance, AccountID> {
        pub fn get_high_price() {

        }
    }

    // type MarketDetail = Vec<KittyIndex>;

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_balances::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Randomness: Randomness<Self::Hash, Self::BlockNumber>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    // Pallets use events to inform users when important changes are made.
    // https://substrate.dev/docs/en/knowledgebase/runtime/events
    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        KittyCreate(T::AccountId, KittyIndex),
        KittyTransfer(T::AccountId, T::AccountId, KittyIndex),
    }


    #[pallet::storage]
    #[pallet::getter(fn kitties_count)]
    pub type KittiesCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn kitties)]
    // pub type Kitties<T: Config> = StorageMap<_, Blake2_128Concat, KittyIndex, Option<Kitty<T::AccountId>>, ValueQuery>;
    pub type Kitties<T: Config> = StorageMap<_, Blake2_128Concat, KittyIndex, Kitty<T::Balance>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn owner)]
    pub type Owner<T: Config> = StorageMap<_, Blake2_128Concat, KittyIndex, Option<T::AccountId>, OptionQuery>;

    // bid是买价，是一个vec，里面包含了很多的购买价格
    #[pallet::storage]
    #[pallet::getter(fn market_bid)]
    pub type MarketBid<T: Config> = StorageMap<_, Blake2_128Concat, KittyIndex, Option<MarketBidDetail<T::Balance, T::AccountId>>, OptionQuery>;

    // ask是卖价 不知道v2怎么定义一个Vec，这里就用一个固定key的map里面存个Vec吧
    // v1可以这么用
    // decl_storage! {
    // 	trait Store for Module<T: Config> as VecSet {
    // 		// The set of all members. Stored as a single vec
    // 		Members get(fn members): Vec<T::AccountId>;
    // 	}
    // }
    #[pallet::storage]
    #[pallet::getter(fn market_ask)]
    pub type MarketAsk<T: Config> = StorageMap<_, Blake2_128Concat, u8, Option<Vec<KittyIndex>>, ValueQuery>;

    // 注意: ValueQuery 和 OptionQuery 差别
    // 默认为ValueQuery
    // ValueQuery默认返回Some(v)
    // let now_count = Self::kitties_count()

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        KittiesCountOverflow,
        NotOwner,
        SameParentIndex,
        InvalidKittyIndex,
        KittyExisted,
    }

    #[pallet::call]
    impl<T:Config> Pallet<T> {
        #[pallet::weight(0)]
        pub fn create(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // let kitty_id = match Self::kitties_count() {
            //     Some(id) => {
            //         ensure!(id != KittyIndex::max_value(), Error::<T>::KittiesCountOverflow);
            //         id
            //     },
            //     None => 0
            // };

            // 生成一个猫的id
            let kitty_id = Self::get_kitty_index().unwrap();
            // 保险，保证新猫的id是干净的
            ensure!(!Kitties::<T>::contains_key(&kitty_id), Error::<T>::KittyExisted);
            // 保险，保证猫没有主人
            ensure!(!Owner::<T>::contains_key(&kitty_id), Error::<T>::KittyExisted);

            let dna = Self::random_value(&who);

            let kitty_obj = Kitty {
                price: 0u8.into(),
                for_sale: false,
                id: kitty_id,
                dna
            };

            // 写入猫信息
            Kitties::<T>::insert(kitty_id, kitty_obj);

            // 为猫设置主人
            Owner::<T>::insert(kitty_id, Some(&who));

            // let kitty_id_now = Self::kitties_count();
            // let new_id = kitty_id_now.checked_add(1).ok_or(Error::<T>::KittiesCountOverflow)?;
            let new_id = Self::next_kitty_index().unwrap();
            KittiesCount::<T>::put(new_id);

            Self::deposit_event(Event::KittyCreate(who, kitty_id));

            Ok(())
        }

        #[pallet::weight(0)]
        pub fn set_price(origin: OriginFor<T>, kitty_id: KittyIndex, new_price: T::Balance, for_sale: bool) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Kitties::<T>::contains_key(&kitty_id), Error::<T>::InvalidKittyIndex);

            let owner = Self::owner(kitty_id).ok_or(Error::<T>::NotOwner)?;

            ensure!(Some(who) == owner, Error::<T>::NotOwner);

            let mut kitty_obj = Self::kitties(kitty_id);
            kitty_obj.price = new_price;
            kitty_obj.for_sale = for_sale;

            Kitties::<T>::insert(kitty_id, kitty_obj);

            // 然后要把这个猫推向市场
            if for_sale {
                // 推向市场
                let _ = Self::add_kitty_to_ask_market(kitty_id);
            }

            Ok(())
        }

        #[pallet::weight(0)]
        pub fn transfer(origin: OriginFor<T>, new_owner: <T::Lookup as StaticLookup>::Source, kitty_id: KittyIndex) ->
        DispatchResult
        {
            let who = ensure_signed(origin)?;
            let new_owner = T::Lookup::lookup(new_owner)?;

            ensure!(Kitties::<T>::contains_key(&kitty_id), Error::<T>::InvalidKittyIndex);

            let owner = Self::owner(kitty_id).ok_or(Error::<T>::NotOwner)?;

            ensure!(Some(who.clone()) == owner, Error::<T>::NotOwner);

            Owner::<T>::insert(kitty_id, Some(new_owner.clone()));

            // let kitty_obj = Kitties::<T>::get(&kitty_id);

            // ensure!(who.clone() == kitty_obj.owner, Error::<T>::NotOwner);

            // ensure!(Some(who.clone()) == Owner::<T>::get(kitty_id), Error::<T>::NotOwner);

            // Owner::<T>::insert(kitty_id, Some(new_owner.clone()));

            // Kitties::<T>::mutate(&kitty_id, |c| {
            //     c.owner = new_owner.clone();
            // });

            Self::deposit_event(Event::KittyTransfer(who, new_owner, kitty_id));

            Ok(())
        }

        #[pallet::weight(0)]
        pub fn breed(origin: OriginFor<T>, kitty_id_1: KittyIndex, kitty_id_2: KittyIndex)
                     -> DispatchResult
        {
            let who = ensure_signed(origin)?;
            ensure!(kitty_id_1 != kitty_id_2, Error::<T>::SameParentIndex);
            ensure!(Kitties::<T>::contains_key(&kitty_id_1), Error::<T>::InvalidKittyIndex);
            ensure!(Kitties::<T>::contains_key(&kitty_id_2), Error::<T>::InvalidKittyIndex);

            // 我的存储类型Kitties并不是Option类型，所以没有ok_or方法
            // let kitty1 = Self::kitties(kitty_id_1).ok_or(Error::<T>::InvalidKittyIndex)?;
            // let kitty2 = Self::kitties(kitty_id_2).ok_or(Error::<T>::InvalidKittyIndex)?;

            let kitty1 = Self::kitties(kitty_id_1);
            let kitty2 = Self::kitties(kitty_id_2);

            // let kitty1 = Kitties::<T>::get(&kitty_id_1);
            // let kitty2= Kitties::<T>::get(&kitty_id_2);


            let kitty_id = Self::get_kitty_index().unwrap();

            let dna_1 = kitty1.dna;
            let dna_2 = kitty2.dna;

            let selector = Self::random_value(&who);
            let mut new_dna = [0u8; 16];

            for i in 0..dna_1.len() {
                new_dna[i] = (selector[i] & dna_1[i]) | (!selector[i] & dna_2[i]);
            }

            let kitty_obj = Kitty {
                price: 0u8.into(),
                for_sale: false,
                id: kitty_id,
                dna: new_dna
            };

            Kitties::<T>::insert(kitty_id, kitty_obj);

            Owner::<T>::insert(kitty_id, Some(who.clone()));

            let new_id = Self::next_kitty_index().unwrap();
            KittiesCount::<T>::put(new_id);
            // KittiesCount::<T>::put(kitty_id + 1);

            Self::deposit_event(Event::KittyCreate(who, kitty_id));

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn random_value(sender: &T::AccountId) -> [u8; 16] {
            let payload = (
                T::Randomness::random_seed(),
                &sender,
                <frame_system::Pallet<T>>::extrinsic_index(),
            );
            payload.using_encoded(blake2_128)
        }

        fn add_kitty_to_ask_market(kitty: KittyIndex) -> Result<(), Error::<T>> {
            let mut market_ask: Vec<KittyIndex> = match Self::market_ask(1) {
                Some(ask) => ask,
                None => vec![],
            };

            match market_ask.binary_search(&kitty) {
                Ok(_) => Ok(()),
                Err(index) => {
                    market_ask.insert(index,kitty.clone());
                    MarketAsk::<T>::insert(1, Some(market_ask));
                    Ok(())
                }
            }
        }

        fn next_kitty_index() -> Result<u32, Error::<T>> {
            let kitty_index = Self::kitties_count();
            let new_id = kitty_index.checked_add(1).ok_or(Error::<T>::KittiesCountOverflow)?;
            Ok(new_id)
        }

        fn get_kitty_index() -> Result<u32, Error::<T>> {
            // match Self::kitties_count() {
            //     Some(id) => {
            //         ensure!(id != KittyIndex::max_value(), Error::<T>::KittiesCountOverflow);
            //         Ok(id)
            //     },
            //     None => Ok(0)
            // }
            let kitty_index = Self::kitties_count();
            let _new_id = kitty_index.checked_add(1).ok_or(Error::<T>::KittiesCountOverflow)?;
            Ok(kitty_index)
        }
    }
}
