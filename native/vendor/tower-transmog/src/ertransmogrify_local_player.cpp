#include <mutex>
#include <array>
#include <algorithm>
#include "ertransmogrify_local_player.hpp"
#include "ertransmogrify_shop.hpp"
#include "utils/players.hpp"

#include <spdlog/spdlog.h>

#include <elden-x/params.hpp>

#include <utility>

using namespace std;

static constexpr int transmog_sfx_id = 8020;

/**
 * Get the player's displayed armor before transmog is applied
 */
static pair<ertransmogrify::vfx::player_state_st, bool> get_default_player_state(
    er::CS::PlayerIns *player) {
    bool ignore_arms_transmog = false;

    auto &gear_param_ids = player->game_data->equip_game_data.chr_asm.gear_param_ids;

    // If the player isn't using transmog, default to their actual armor.
    auto state = ertransmogrify::vfx::player_state_st{
        .head_protector_id = gear_param_ids.head_protector_id,
        .chest_protector_id = gear_param_ids.chest_protector_id,
        .arms_protector_id = gear_param_ids.arms_protector_id,
        .legs_protector_id = gear_param_ids.legs_protector_id};

    // If any other transform effects are present other than transmog (e.g. the DLC transformation
    // items), override the armor.
    for (auto entry = player->special_effects->head; entry; entry = entry->next) {
        if (entry->param_id >= ertransmogrify::vfx::transmog_vfx_speffect_start_id &&
            entry->param_id < ertransmogrify::vfx::transmog_vfx_speffect_end_id) {
            continue;
        }

        if (entry->param->vfxId != -1) {
            auto &vfx = er::param::SpEffectVfxParam[entry->param->vfxId].first;
            if (vfx.transformProtectorId != -1) {
                if (vfx.isFullBodyTransformProtectorId) {
                    state.chest_protector_id =
                        vfx.transformProtectorId + ertransmogrify::vfx::chest_protector_offset;
                    state.arms_protector_id =
                        vfx.transformProtectorId + ertransmogrify::vfx::arms_protector_offset;
                    state.legs_protector_id =
                        vfx.transformProtectorId + ertransmogrify::vfx::legs_protector_offset;
                } else if (vfx.isArmsTransformProtectorId) {
                    state.arms_protector_id =
                        vfx.transformProtectorId + ertransmogrify::vfx::arms_protector_offset;

                    // Super special case: arm transforms are used for the singular purpose of
                    // hiding armor while beast claws or DLC fists are equipped, and should take
                    // priority over transmog selections.
                    ignore_arms_transmog = true;
                } else {
                    state.head_protector_id =
                        vfx.transformProtectorId + ertransmogrify::vfx::head_protector_offset;
                }
            }
        }
    }

    return {state, ignore_arms_transmog};
}

static std::mutex selection_mutex;
static std::array<int,4> selection{-1,-1,-1,-1};
extern "C" __declspec(dllexport) bool tower_transmog_set(const int *ids) {
    if (!ids) return false;
    for (int i=0;i<4;i++) {
        if (ids[i]==-1) continue;
        auto [row,found]=er::param::EquipParamProtector[ids[i]];
        if (!found || row.protectorCategory!=i) return false;
    }
    std::lock_guard lock(selection_mutex);
    std::copy(ids,ids+4,selection.begin());
    return true;
}
ertransmogrify::vfx::player_state_st ertransmogrify::local_player::get_local_player_state(er::CS::PlayerIns *player) {
    if (!player) return {};
    std::array<int,4> chosen;
    {std::lock_guard lock(selection_mutex);chosen=selection;}
    if (std::all_of(chosen.begin(),chosen.end(),[](int id){return id<0;})) return {};
    auto [state,ignore_arms]=get_default_player_state(player);
    if(chosen[0]>=0)state.head_protector_id=chosen[0];
    if(chosen[1]>=0)state.chest_protector_id=chosen[1];
    if(chosen[2]>=0&&!ignore_arms)state.arms_protector_id=chosen[2];
    if(chosen[3]>=0)state.legs_protector_id=chosen[3];
    return state;
}
