cmake_minimum_required(VERSION 3.24 FATAL_ERROR)
project(tfhe_cuda_backend LANGUAGES CXX CUDA)

# See if the minimum CUDA version is available. If not, only enable documentation building.
set(MINIMUM_SUPPORTED_CUDA_VERSION 10.0)
include(CheckLanguage)
# See if CUDA is available
check_language(CUDA)
# If so, enable CUDA to check the version.
if(CMAKE_CUDA_COMPILER)
  enable_language(CUDA)
endif()
# If CUDA is not available, or the minimum version is too low do not build
if(NOT CMAKE_CUDA_COMPILER)
  message(FATAL_ERROR "Cuda compiler not found.")
endif()

if(CMAKE_CUDA_COMPILER_VERSION VERSION_LESS ${MINIMUM_SUPPORTED_CUDA_VERSION})
  message(FATAL_ERROR "CUDA ${MINIMUM_SUPPORTED_CUDA_VERSION} or greater is required for compilation.")
endif()
# Get CUDA compute capability
set(OUTPUTFILE ${CMAKE_CURRENT_SOURCE_DIR}/cuda_script) # No suffix required
set(CUDAFILE ${CMAKE_CURRENT_SOURCE_DIR}/check_cuda.cu)
execute_process(COMMAND nvcc -lcuda ${CUDAFILE} -o ${OUTPUTFILE})
execute_process(
  COMMAND ${OUTPUTFILE}
  RESULT_VARIABLE CUDA_RETURN_CODE
  OUTPUT_VARIABLE ARCH)
file(REMOVE ${OUTPUTFILE})

if(${CUDA_RETURN_CODE} EQUAL 0)
  set(CUDA_SUCCESS "TRUE")
else()
  set(CUDA_SUCCESS "FALSE")
endif()

if(${CUDA_SUCCESS})
  message(STATUS "CUDA Architecture: ${ARCH}")
  message(STATUS "CUDA Version: ${CUDA_VERSION_STRING}")
  message(STATUS "CUDA Path: ${CUDA_TOOLKIT_ROOT_DIR}")
  message(STATUS "CUDA Libraries: ${CUDA_LIBRARIES}")
  message(STATUS "CUDA Performance Primitives: ${CUDA_npp_LIBRARY}")
else()
  message(WARNING ${ARCH})
endif()

if(NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE Release)
endif()

# Add OpenMP support
find_package(OpenMP REQUIRED)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra")
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS}  -g")
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} ${OpenMP_CXX_FLAGS}")
set(CMAKE_CUDA_FLAGS "${CMAKE_CUDA_FLAGS} -Xcompiler ${OpenMP_CXX_FLAGS}")
set(CMAKE_CUDA_ARCHITECTURES native)
if(NOT CUDA_NVCC_FLAGS)
  set(CUDA_NVCC_FLAGS -arch=sm_70)
endif()

# in production, should use -arch=sm_70 --ptxas-options=-v to see register spills -lineinfo for better debugging
set(CMAKE_CUDA_FLAGS
    "${CMAKE_CUDA_FLAGS} -ccbin ${CMAKE_CXX_COMPILER} -O3 \
  -std=c++17 --no-exceptions  --expt-relaxed-constexpr -rdc=true \
  --use_fast_math -Xcompiler -fPIC")

set(INCLUDE_DIR include)

add_subdirectory(src)
target_include_directories(tfhe_cuda_backend PRIVATE ${INCLUDE_DIR})

# This is required for rust cargo build
install(TARGETS tfhe_cuda_backend DESTINATION .)
install(TARGETS tfhe_cuda_backend DESTINATION lib)

# Define a function to add a lint target.
find_file(CPPLINT NAMES cpplint cpplint.exe)
if(CPPLINT)
  # Add a custom target to lint all child projects. Dependencies are specified in child projects.
  add_custom_target(all_lint)
  # Don't trigger this target on ALL_BUILD or Visual Studio 'Rebuild Solution'
  set_target_properties(all_lint PROPERTIES EXCLUDE_FROM_ALL TRUE)
  # set_target_properties(all_lint PROPERTIES EXCLUDE_FROM_DEFAULT_BUILD TRUE)
endif()

enable_testing()
