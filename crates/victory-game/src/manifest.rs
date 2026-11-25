//! Package manifests for verifying user-supplied Xbox Series game installs.
//!
//! Adapted from Leonida (`game/xbox-1.0.2.1.json`, GPL-3.0-or-later).

use std::path::Path;

/// A single expected file in a retail Xbox Series package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackageFile {
    /// Relative path using `/` separators.
    pub path: &'static str,
    /// Exact byte length of the file.
    pub size: u64,
    /// Lowercase hex SHA-256 digest.
    pub sha256: &'static str,
}

/// Package manifest for a specific title version.
#[derive(Debug, Clone, Copy)]
pub struct PackageManifest {
    pub title: &'static str,
    pub version: &'static str,
    pub expected_exe: &'static str,
    pub files: &'static [PackageFile],
}

/// Result of checking a game directory against a [`PackageManifest`].
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PackageReport {
    pub present: usize,
    pub missing: Vec<&'static str>,
    pub size_mismatches: Vec<(&'static str, u64, u64)>,
}

impl PackageReport {
    /// Returns `true` when every manifest file exists with its expected byte size.
    pub fn is_complete(&self) -> bool {
        self.missing.is_empty() && self.size_mismatches.is_empty()
    }
}

/// Verifies file presence and byte sizes in `root` against `manifest`.
pub fn verify_package_dir(root: &Path, manifest: &PackageManifest) -> PackageReport {
    let mut report = PackageReport::default();
    for entry in manifest.files {
        let path = root.join(entry.path);
        match std::fs::metadata(&path) {
            Ok(meta) => {
                if meta.len() == entry.size {
                    report.present += 1;
                } else {
                    report
                        .size_mismatches
                        .push((entry.path, entry.size, meta.len()));
                }
            }
            Err(_) => report.missing.push(entry.path),
        }
    }
    report
}

/// Xbox Series package manifest for version `1.0.2.1`.
pub const XBOX_1_0_2_1: PackageManifest = PackageManifest {
    title: "Xbox Series package",
    version: "1.0.2.1",
    expected_exe: "game_scarlett_master.exe",
    files: XBOX_1_0_2_1_FILES,
};

const XBOX_1_0_2_1_FILES: &[PackageFile] = &[
    PackageFile { path: "Bink2Scarlett.dll", size: 255_488, sha256: "9eaf045d004cd7d6a8382aaff0c6ea23a29fec4ef8bfacd726b54f5bc26fafd0" },
    PackageFile { path: "Catalog000.bin", size: 815_104, sha256: "4afbdbe22aad2bc338a5ba1ffe83cccb96e072eb4e92f6a5dd90b49b2e227c59" },
    PackageFile { path: "MicrosoftGame.Config", size: 2_542, sha256: "d0d5e4777460045916046619cd0fd433aa251d4a74f4d91e83987588f907f80c" },
    PackageFile { path: "Update.alignmentchunk", size: 1, sha256: "5feceb66ffc86f38d952786c6d696c79c2dbc239dd4e91b46729d73a27fb57e9" },
    PackageFile { path: "XCurl.dll", size: 199_152, sha256: "d11ba4e4a07f42f8d80c48553c5c2588ab4114ecdbe023305f5f8fa8262131f8" },
    PackageFile { path: "XboxBvhBuilder.dll", size: 884_224, sha256: "99f803ac22c01ef329dddb1d40be67238d2a812d01e8d040b3cb5135bc23707e" },
    PackageFile { path: "amd_fidelityfx_loader_xbox.dll", size: 13_312, sha256: "431bdd2e26c0c8214fd7601140eb9fe6d8a249f5000539453a9a9b6a0b7c0ee8" },
    PackageFile { path: "amd_fidelityfx_upscaler_xbox.dll", size: 786_432, sha256: "faff02f294a7e9149ce079dffe3dedbcabe99b1f180ab14222aa8689e00a75fa" },
    PackageFile { path: "anim.rpf", size: 4_407_626_384, sha256: "7f5c53162ab66a8915dfc95ddf22aad9055635b6ddd3cffc40f431e24ea1c189" },
    PackageFile { path: "appdata0.rpf", size: 8_880, sha256: "d9d4d2ddcd23e73a8862fbb5d5b53ca259e454f90319910d88f3a5c8da212535" },
    PackageFile { path: "appxmanifest.xml", size: 3_394, sha256: "662a05d05f8449b94b2ffc052f2a9ad2c090b3708d5a274fc9b2db0e14b03a34" },
    PackageFile { path: "audio.rpf", size: 28_543_454_672, sha256: "883581135f2fb0ba7ec5068da6f89363951400d4d3f9c0b99c06ac7eef24134a" },
    PackageFile { path: "common.rpf", size: 6_642_192, sha256: "31dc6aa5394dd8cd3c83b90c9661f37325f22851cc6d5d7150e11041c7618ced" },
    PackageFile { path: "concrt140.dll", size: 298_992, sha256: "554168b99726cbe982b542ec97c493f48483b20a4ea4e54e102ba3a0d098e75f" },
    PackageFile { path: "data.rpf", size: 481_544_304, sha256: "e5af629ef458bb7be55cae3dc890c69613c8028060ac765d54a4ccd73991640c" },
    PackageFile { path: "data/ttdb/graphics/ttdb_graphics.ttpf", size: 34_411_776, sha256: "0b23f3f4ea9f63d8bb8abd835fb15b584fbc63a2b2f9cdbb6f675d78db16cb7e" },
    PackageFile { path: "data/ttdb/interiors/ttdb_interiors_0_3.ttpf", size: 883_202_048, sha256: "5f109fcd493235e271c0106c9da92a27de1e48d809a18a5d2af8c2183bb23516" },
    PackageFile { path: "data/ttdb/interiors/ttdb_interiors_1_3.ttpf", size: 898_113_536, sha256: "3598c14ad6b1df887b21b3f8685be2ab8a060264ca3f8db3829d3f4bc0863b8c" },
    PackageFile { path: "data/ttdb/interiors/ttdb_interiors_2_3.ttpf", size: 881_431_552, sha256: "b8616380028bb8fe027c46c0a225793e5814a6639afb9d90dede4acabfde2004" },
    PackageFile { path: "data/ttdb/interiors/ttdb_interiors_3_3.ttpf", size: 900_875_008, sha256: "d284d983873bc726b68c36ea85d0fd8c3a7c8ae58be049c981218eed00f614ba" },
    PackageFile { path: "data/ttdb/lighting/lighting.ttpf", size: 151_531_776, sha256: "87727791b5026439d8f0b3bcceeae02b074fda9670270cac9bc248ffac16e3ca" },
    PackageFile { path: "data/ttdb/maps/ttdb_maps_0_7.ttpf", size: 1_765_679_616, sha256: "d672e99c248b943c3a3acb694eb16fe6236d3d9d04c86451cf68259ca68d4409" },
    PackageFile { path: "data/ttdb/maps/ttdb_maps_1_7.ttpf", size: 1_746_187_776, sha256: "df9f32bb42168ac0abf9e44573d837186107d5d3dc926a0cf597750f6cae112f" },
    PackageFile { path: "data/ttdb/maps/ttdb_maps_2_7.ttpf", size: 1_818_806_528, sha256: "2ca6dd82a842990b6a2c3440936b443808a6d98bf6b540048978591bdc23b7c0" },
    PackageFile { path: "data/ttdb/maps/ttdb_maps_3_7.ttpf", size: 1_787_349_760, sha256: "f90577103ef7986ba7ae4d190782bf73378d4e0e5be6f9664ea3fc3f3088255e" },
    PackageFile { path: "data/ttdb/maps/ttdb_maps_4_7.ttpf", size: 1_791_670_784, sha256: "ccc48a1662467b78423d3f83220e3bc4f4ae36c32a873d09344d48c1faab9760" },
    PackageFile { path: "data/ttdb/maps/ttdb_maps_5_7.ttpf", size: 1_840_029_440, sha256: "16229da15fb88d8a76dceadfb0cacb852144878960a7584b3345051d351c9c3c" },
    PackageFile { path: "data/ttdb/maps/ttdb_maps_6_7.ttpf", size: 1_745_923_072, sha256: "b852c332f5474a3631e13c708f2233b27668065d8a9ee751155efb98e27a3a6e" },
    PackageFile { path: "data/ttdb/maps/ttdb_maps_7_7.ttpf", size: 1_755_748_608, sha256: "27be3cf33f23326b9dec29692788e8ee75f73ce0050c23307600dbd067964de5" },
    PackageFile { path: "data/ttdb/peds/decorations_0_3.ttpf", size: 494_316_032, sha256: "31246c91ea208b2b58809cb5bd82c5e0c044c88ae4198831aab727584675b0f3" },
    PackageFile { path: "data/ttdb/peds/decorations_1_3.ttpf", size: 510_001_152, sha256: "baf71941408cf65f9aed0335a737783f62ca1899f863322d89f9a990bcb7f0e3" },
    PackageFile { path: "data/ttdb/peds/decorations_2_3.ttpf", size: 491_507_456, sha256: "5c622d5a00f0f84125edc3c84061a4f97d57dd695ec6a286657f7135445cb3c6" },
    PackageFile { path: "data/ttdb/peds/decorations_3_3.ttpf", size: 497_391_360, sha256: "aeabd38813f8af5582aad43e84b0a0262ef187d03115483764433ad5b1f848df" },
    PackageFile { path: "data/ttdb/peds/peds_loose.ttpf", size: 135_676_160, sha256: "0c7ad96cf458be86c38f1cbdbc067f93ea444eda4a88c72b16d0ecdfcc91e311" },
    PackageFile { path: "data/ttdb/peds/ttdb_peds_0_3.ttpf", size: 953_116_416, sha256: "73891cde5598e26e4dda53971d45d91c749e3d54dc2afe93bee9885344ea3dde" },
    PackageFile { path: "data/ttdb/peds/ttdb_peds_1_3.ttpf", size: 965_556_736, sha256: "b37dcb845e9143409879de2b7fddf82d38a736d07041082648a7f718f9583faa" },
    PackageFile { path: "data/ttdb/peds/ttdb_peds_2_3.ttpf", size: 937_699_072, sha256: "8ebca98d174266f5cf18cb50cc3c5ca5e7ecdbc84604f352bb6ff4fb05f9c2ef" },
    PackageFile { path: "data/ttdb/peds/ttdb_peds_3_3.ttpf", size: 957_291_520, sha256: "3f9d03d67978f5454a927266c7f866f20c700bb753a83f5cbc7182a398618eeb" },
    PackageFile { path: "data/ttdb/props/octa_imposters.ttpf", size: 983_620_352, sha256: "03a7d6dd60da98871228b1ec06a93d4a00387143dd20f123c0408087385f203c" },
    PackageFile { path: "data/ttdb/props/ttdb_props_0_3.ttpf", size: 1_304_458_240, sha256: "8e620456a4a296faf8be5632a8d929df590be1efeee7d5ce34059726a0ac65f4" },
    PackageFile { path: "data/ttdb/props/ttdb_props_1_3.ttpf", size: 1_256_908_800, sha256: "3cba53b0fa123e39597e8331fe4b432294fcf32baebb94f9db47a52f1b46ac4d" },
    PackageFile { path: "data/ttdb/props/ttdb_props_2_3.ttpf", size: 1_239_990_784, sha256: "6999079496270431dfb6df46a6406f721cfb4331e418bab6c6facb962f4d288f" },
    PackageFile { path: "data/ttdb/props/ttdb_props_3_3.ttpf", size: 1_328_911_616, sha256: "7633d784b6e77258f8d576777633fc72eebbff004c1640ee27d53773ced83639" },
    PackageFile { path: "data/ttdb/terrain/ttdb_terrain_0_7.ttpf", size: 2_775_084_288, sha256: "6de6efac9d3d7b862548b3d0aee352b086c2e01ebcf8cda8ede617afb495187f" },
    PackageFile { path: "data/ttdb/terrain/ttdb_terrain_1_7.ttpf", size: 2_739_444_480, sha256: "e1f5139ed75d6781754737cf277981aa5c3afea5d2d06e7dcd57a36fb4ff505c" },
    PackageFile { path: "data/ttdb/terrain/ttdb_terrain_2_7.ttpf", size: 2_780_145_920, sha256: "40b157a43413359fafef667aab1a6ec53d2c078c50fe24292678c3d1b33ede9a" },
    PackageFile { path: "data/ttdb/terrain/ttdb_terrain_3_7.ttpf", size: 2_718_679_808, sha256: "5799083008834d706e6b3606b647abfbfa6068574c96f3803ad957013bb4eb1a" },
    PackageFile { path: "data/ttdb/terrain/ttdb_terrain_4_7.ttpf", size: 2_741_339_648, sha256: "18c6ce167de25a1cea110401da7c2ce679106f7766df0b7af2668cba7a1ab25b" },
    PackageFile { path: "data/ttdb/terrain/ttdb_terrain_5_7.ttpf", size: 2_674_379_008, sha256: "60bcba6a7f3a658cec51232cc48098cf86a6e54d37f21ab9401b2a76e2a6ea53" },
    PackageFile { path: "data/ttdb/terrain/ttdb_terrain_6_7.ttpf", size: 2_688_808_960, sha256: "553961eff42fb8deb7cf7330d303e843b88ab29c87ab30b85cdd0c0501d1fc14" },
    PackageFile { path: "data/ttdb/terrain/ttdb_terrain_7_7.ttpf", size: 2_828_296_704, sha256: "dc887908067d49d9bd05c40029f7248a6bbf76fe79f34abea0199dd6a7edf244" },
    PackageFile { path: "data/ttdb/terrain/ttdb_terrain_image_based_0_1.ttpf", size: 1_946_070_528, sha256: "30a3020d15d4d159b06aa9fd31508a534c79d9990de215bf20d2572ec2713995" },
    PackageFile { path: "data/ttdb/terrain/ttdb_terrain_image_based_1_1.ttpf", size: 1_890_978_048, sha256: "1e0afca572034adebc2899e733546fb12a620acd10b32a4c21fbcc5926abed69" },
    PackageFile { path: "data/ttdb/ui/platform.ttpf", size: 222_976, sha256: "3e64d01e062ed55d290aaf3a7b9694d562a8591cf461e638dd135d388d4b5e5c" },
    PackageFile { path: "data/ttdb/ui/ttdb_ui.ttpf", size: 845_115_648, sha256: "dc983ed7f6b78d1693fa5757f17c03bafb790e851594fdf4d412579b61d867f6" },
    PackageFile { path: "data/ttdb/vehicles/ttdb_vehicles.ttpf", size: 195_418_624, sha256: "02add8dc1c4f8f485ef8abc5b4480b746e24fa4a546eb488ec37ff1b4d3c4e34" },
    PackageFile { path: "data/ttdb/vehicles/ttdb_vehicles_loose_0_1.ttpf", size: 838_474_496, sha256: "0e90a8dbda5e32fb6c47a4ecabf821451a59abd349755c8030c380d4a9e9f834" },
    PackageFile { path: "data/ttdb/vehicles/ttdb_vehicles_loose_1_1.ttpf", size: 890_206_208, sha256: "a3aae83237fedd1f7c32e8ab2adc8bf9a87d8b019a2242c5181547714b00f8cc" },
    PackageFile { path: "data/ttdb/vfx/ttdb_vfx.ttpf", size: 238_897_664, sha256: "7625184c4e9c899f3b708b88d61fa2e12b836db63640385cecd750cb30c8f26e" },
    PackageFile { path: "data/ttdb/weapons/ttdb_weapons.ttpf", size: 1_421_518_080, sha256: "41c4fc92143fc26e44dcb4ca93142b914f9ac1188f9b53cbf8db27c6a97b6d4c" },
    PackageFile { path: "data/ttdb/weapons/weapon_liveries.ttpf", size: 739_758_848, sha256: "7cae5a1ce890e7a4d452387f4dcb4ebde8e3e3b905409d7ed33903203a348ba5" },
    PackageFile { path: "gameos.xvd", size: 348_069_888, sha256: "d02ff50ae9164e540ce55b60b2f128b5c78bab6bace6e5eee46e584438d9af36" },
    PackageFile { path: "layout_3202719e-b4b4-421c-9f8f-70d1c4a7d60b_xs.xml", size: 13_698, sha256: "2eeb9b05047f83c8bc82fcee7646e7d675266f4545f940593a6b56e92358663d" },
    PackageFile { path: "levels.rpf", size: 76_155_065_056, sha256: "9d1b1bd70659a32dc6259fc890dbc3e167dd3f9253f41000a3c9aa7c17539c47" },
    PackageFile { path: "libHttpClient.GDK.dll", size: 975_856, sha256: "14d97b3d071e0ce461e758dbcf1511941ae5c0bdd6e32ed33cb87aaae0414d3a" },
    PackageFile { path: "microsoft_gameconfig.bin", size: 8_192, sha256: "27735d6ca90ff4eb8f432e3d3e701fd85f75303f1adade2cdbc4bef165159a0a" },
    PackageFile { path: "movies.rpf", size: 8_832_481_776, sha256: "a0647c0325333dd42b25b00d85ee9b3915a55ce99127b97ef00662ef4866cba4" },
    PackageFile { path: "msvcp140.dll", size: 565_232, sha256: "6036fdb3ee3a5813ef0b17462219ebcee5f06b46ef738bacde07aa797e6d2b75" },
    PackageFile { path: "msvcp140_1.dll", size: 26_104, sha256: "c56b530e0894d9c4d11b12d389786fef40463dbd81d4438b8414f909d6226211" },
    PackageFile { path: "msvcp140_2.dll", size: 187_888, sha256: "276813083c55c079120e0db49898088bb0c8d1c912ed59ec0d00fedf56a694a8" },
    PackageFile { path: "msvcp140_atomic_wait.dll", size: 57_312, sha256: "b7f879662095f71dc08efcbc0d24ac3c0ff2dde8d475f92f5c5646a39bf8f2c3" },
    PackageFile { path: "msvcp140_codecvt_ids.dll", size: 22_008, sha256: "89cf5b07b98709277e8511d9a402ce72279e8df3ae24a8dd641ab0265c799a0c" },
    PackageFile { path: "navmeshes.rpf", size: 6_707_512_496, sha256: "30f8d4ebc0fd1d9f786cd50627a7bd3bd537288707606a18815a6440da8e3109" },
    PackageFile { path: "pack/core_bvh4/core_bvh4.rpf", size: 8_297_921_520, sha256: "a316887568569127e4ebb0053de11b9cefa4305ce6199fb85b5acc62bfa2ff5a" },
    PackageFile { path: "pack/core_licenses/core_licenses_data.rpf", size: 13_536, sha256: "6ca2574aa699b4583f110c6b3e3ae605a6a0ca8c36d65769f76e3beb58e93751" },
    PackageFile { path: "pack/core_restofworld/core_restofworld.rpf", size: 1_295_760_752, sha256: "a28e53eee58e0b7541bee699ca0136d7a8402934984fe1d12d573905a24da5f0" },
    PackageFile { path: "pack/core_restofworld/data/ttdb/peds/ttdb_peds.ttpf", size: 24_529_408, sha256: "b3514f9b1fe258141f7b5d840fc753e980a2af59749a1f8d4de0867e93cf5c97" },
    PackageFile { path: "pack/core_restofworld/data/ttdb/props/ttdb_props.ttpf", size: 35_286_784, sha256: "eaf45801f0a0b0407ff41d5e840a27f9cfce144e1a4798ca293003a484e2c0ee" },
    PackageFile { path: "pack/core_restofworld/data/ttdb/textures_loose.ttpf", size: 21_081_856, sha256: "81da4d1904dd038ea59fb5d7b851e2a15b1906a5ff3017f1bd15dd617d46c3dc" },
    PackageFile { path: "pack/core_restofworld/data/ttdb/ui/ttdb_ui.ttpf", size: 4_729_600, sha256: "9df804569a50dfe7d1a0f8ef71fa9a117a370616159489e9028f5acfd5118d66" },
    PackageFile { path: "pack/core_restofworld/data/ttdb/vfx/ttdb_vfx.ttpf", size: 3_129_600, sha256: "6434287056329e60a4ca7ae26fabf91dc3ccf3b738130033400fb88ab42da8cf" },
    PackageFile { path: "pack/core_restofworld001/core_restofworld001.rpf", size: 791_984, sha256: "6eb47c010bb6bf69f50d69675e50415d345468f3a1193fdf7ab4e54a4d7d6c56" },
    PackageFile { path: "pack/core_restofworld001/data/ttdb/ui/ttdb_ui.ttpf", size: 38_656, sha256: "9bfb5f4b85be2f6d1d93b2ed83df94a2021346448816b61fd9c08d7d05b8a43c" },
    PackageFile { path: "pack/shared_minigame000/data/ttdb/maps/ttdb_maps.ttpf", size: 193_522_176, sha256: "873a8c7634eacaa062a1bc48888908f3e2d3196159b9fa39e83eb6cfc18dbb06" },
    PackageFile { path: "pack/shared_minigame000/data/ttdb/terrain/ttdb_terrain_image_based.ttpf", size: 1_427_712, sha256: "0354565963b8472bf2f606dc1a4089c14f45cb5ecd416984f5e4212009eb7e74" },
    PackageFile { path: "pack/shared_minigame000/shared_minigame000.rpf", size: 810_581_008, sha256: "badd8f4b3a4e65bc492d2933d0c82239653467317c606c483e616979a5ec5967" },
    PackageFile { path: "pack/shared_minigame000_bvh4/shared_minigame000_bvh4.rpf", size: 61_987_040, sha256: "9525a81d18b922ba5859643883d466f08442f779b7c92a7eaaefde3766292a96" },
    PackageFile { path: "pack/story000/data/ttdb/maps/ttdb_maps.ttpf", size: 697_378_560, sha256: "8e362c1f39383d6ff97eeffbf963602e9ed2e2b97b707e54a3884fe7c8dd408d" },
    PackageFile { path: "pack/story000/data/ttdb/terrain/ttdb_terrain_image_based.ttpf", size: 222_333_440, sha256: "4794cbff80790c7a0b3d2959f3831b53e90c5b56b9bdb61814f98f29a4e4157f" },
    PackageFile { path: "pack/story000/data/ttdb/vfx/ttdb_vfx.ttpf", size: 3_357_184, sha256: "79612738f51faa724cb6d30ad5465419f3038d4b3498115ec1306b0ad7d71f1c" },
    PackageFile { path: "pack/story000/story000.rpf", size: 20_022_508_672, sha256: "4d7f87158a1be7e5e4f15ef195ddaa804be94d2ecaafaa9172150f7b6b8abf94" },
    PackageFile { path: "pack/story000/story000_script.rpf", size: 78_023_728, sha256: "fb30232fd94ccfdb2b85bb8deae860b7c449348f2b4a8c38d9395de1ad00b8fb" },
    PackageFile { path: "pack/story000_bvh4/story000_bvh4.rpf", size: 5_632, sha256: "b9f86219f835c769ca86274f933ab35423af45dad33f130d5c5e4727450ffd24" },
    PackageFile { path: "pack/story000_restofworld/story000_restofworld.rpf", size: 198_354_992, sha256: "f972251e4369d634419e33d2b2b6196f310e70af521dd0c9f00025d0c1c0e6ca" },
    PackageFile { path: "packs.rpf", size: 4_798_213_264, sha256: "ed6c55c60112704f7e551cfbb2446f7f069bfd7caa1c257bc427595cf7434651" },
    PackageFile { path: "resources.pri", size: 1_152, sha256: "1a91d1a8de2fa9bffafcfbdb0f4c1a8a02294647eec71385baa70b8d1e4a165e" },
    PackageFile { path: "script.rpf", size: 4_358_960, sha256: "c4d7e75f867c8527eee75135cd400e22f7c770c159c6bc2c3f80ec0f6e6a8dbb" },
    PackageFile { path: "shaders.rpf", size: 411_695_600, sha256: "0bc1634594e51ca8e7ef5f2bc6f5b32b081ce779ac094d686de08ecb75dc24a8" },
    PackageFile { path: "textures.rpf", size: 192_585_024, sha256: "dc1957ec85d89b1b55cd1fdc5c3997f48d1e0bff9020ec2842120ab34e71c83e" },
    PackageFile { path: "vccorlib140.dll", size: 336_888, sha256: "79ee6fa49aa2c6f0dfbc1279273fd619e1a9285995ff53a8bc183000c8895b91" },
    PackageFile { path: "vcomp140.dll", size: 72_176, sha256: "91e249c0f66f827bcacd0be4931b1cec3e4907cc05ea0a5bcd83696c70908963" },
    PackageFile { path: "vcruntime140.dll", size: 97_776, sha256: "9385104cc3e031dbbcb63f156134a0f6afff455e0ff6887f5a9e1c5f7e2706b0" },
    PackageFile { path: "vcruntime140_1.dll", size: 37_848, sha256: "0dbd6859f825ab8acab1fa25b6bd1a091d35ca2dbe3185c1187192f2d2ac8154" },
    PackageFile { path: "xbs_logo_150.png", size: 54_536, sha256: "21a652895663671afef4e947cc8ced47ca66ec00f4b27059c17bef6965a506ca" },
    PackageFile { path: "xbs_logo_44.png", size: 7_102, sha256: "072c08f82513a8f6a392dd8be371378333191a4aabbc5b6a1fedfe7dbc850916" },
    PackageFile { path: "xbs_logo_480.png", size: 404_663, sha256: "b5ac80b8d43bc23c9de0593fad04a7eafa5106ed405eeb5db40ad9b37642d535" },
    PackageFile { path: "xbs_logo_store.png", size: 29_356, sha256: "006999d45faf184bf669610f364b9551874d0fdb154e36ce34e622a300d025da" },
    PackageFile { path: "xbs_splash.png", size: 2_507_194, sha256: "0fa110ecac0a5fac8294921132cc8322440bdf93e74e723eb63eae0143d58e58" },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_entries_are_well_formed() {
        assert_eq!(XBOX_1_0_2_1.files.len(), 108);
        for f in XBOX_1_0_2_1.files {
            assert!(!f.path.is_empty());
            assert_eq!(f.sha256.len(), 64, "bad sha256 for {}", f.path);
            assert!(
                f.sha256.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
                "non-hex sha256 for {}",
                f.path
            );
        }
    }

    #[test]
    fn verify_package_dir_reports_missing_and_mismatches() {
        let dir = std::env::temp_dir().join(format!("victory-manifest-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // Create Update.alignmentchunk with exact size (1 byte) and wrong-sized Catalog000.bin.
        std::fs::write(dir.join("Update.alignmentchunk"), [0u8]).unwrap();
        std::fs::write(dir.join("Catalog000.bin"), [0u8; 4]).unwrap();

        let report = verify_package_dir(&dir, &XBOX_1_0_2_1);
        assert_eq!(report.present, 1);
        assert_eq!(report.size_mismatches, vec![("Catalog000.bin", 815_104, 4)]);
        assert_eq!(report.missing.len(), XBOX_1_0_2_1.files.len() - 2);
        assert!(!report.is_complete());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
