#----------------------------------------------------------------
# Generated CMake target import file for configuration "Release".
#----------------------------------------------------------------

# Commands may need to know the format version.
set(CMAKE_IMPORT_FILE_VERSION 1)

# Import target "simpleble::simpleble" for configuration "Release"
set_property(TARGET simpleble::simpleble APPEND PROPERTY IMPORTED_CONFIGURATIONS RELEASE)
set_target_properties(simpleble::simpleble PROPERTIES
  IMPORTED_IMPLIB_RELEASE "${_IMPORT_PREFIX}/lib/simpleble.lib"
  IMPORTED_LOCATION_RELEASE "${_IMPORT_PREFIX}/bin/simpleble.dll"
  )

list(APPEND _cmake_import_check_targets simpleble::simpleble )
list(APPEND _cmake_import_check_files_for_simpleble::simpleble "${_IMPORT_PREFIX}/lib/simpleble.lib" "${_IMPORT_PREFIX}/bin/simpleble.dll" )

# Import target "simpleble::simpleble-c" for configuration "Release"
set_property(TARGET simpleble::simpleble-c APPEND PROPERTY IMPORTED_CONFIGURATIONS RELEASE)
set_target_properties(simpleble::simpleble-c PROPERTIES
  IMPORTED_IMPLIB_RELEASE "${_IMPORT_PREFIX}/lib/simpleble-c.lib"
  IMPORTED_LINK_DEPENDENT_LIBRARIES_RELEASE "simpleble::simpleble"
  IMPORTED_LOCATION_RELEASE "${_IMPORT_PREFIX}/bin/simpleble-c.dll"
  )

list(APPEND _cmake_import_check_targets simpleble::simpleble-c )
list(APPEND _cmake_import_check_files_for_simpleble::simpleble-c "${_IMPORT_PREFIX}/lib/simpleble-c.lib" "${_IMPORT_PREFIX}/bin/simpleble-c.dll" )

# Commands beyond this point should not need to know the version.
set(CMAKE_IMPORT_FILE_VERSION)
