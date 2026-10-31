#include "ertransmogrify_vfx.hpp"
#include "utils/players.hpp"
#include <elden-x/params.hpp>
#include <elden-x/utils/modutils.hpp>
#include <spdlog/spdlog.h>
#include <spdlog/sinks/ostream_sink.h>
#include <filesystem>
#include <fstream>
#include <atomic>
#include <exception>

namespace ertransmogrify::config {bool client_side_only=false;}
static std::atomic<int> status{0};
static std::ofstream log_stream;
extern "C" __declspec(dllexport) int tower_transmog_status(){return status.load();}
extern "C" __declspec(dllexport) bool tower_transmog_init(const char *log_path) {
    int expected=0;
    if(!status.compare_exchange_strong(expected,1))return status==2;
    try {
        log_stream.open(std::filesystem::u8path(log_path),std::ios::app);
        if(!log_stream)throw std::runtime_error("Cannot open transmog log");
        auto logger=std::make_shared<spdlog::logger>("tower-transmog",std::make_shared<spdlog::sinks::ostream_sink_mt>(log_stream));
        spdlog::set_default_logger(logger);logger->flush_on(spdlog::level::info);
        modutils::initialize();er::FD4::find_singletons();players::initialize();
        er::CS::SoloParamRepository::wait_for_params();
        ertransmogrify::vfx::initialize();modutils::enable_hooks();
        status=2;SPDLOG_INFO("Tower armor transmog initialized (integrated engine 1)");return true;
    }catch(const std::exception &e){SPDLOG_ERROR("Transmog unavailable: {}",e.what());status=-1;return false;}
}
